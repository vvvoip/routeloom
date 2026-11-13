import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

enum TunnelState {
  unavailable,
  disconnected,
  connecting,
  connected,
  disconnecting,
  failed,
}

class TunnelProfile {
  TunnelProfile._(this.json);
  final String json;
  factory TunnelProfile.parse(String raw) {
    if (raw.length > 65536) throw const FormatException('Profile too large');
    final value = jsonDecode(raw) as Map<String, dynamic>;
    const keys = {
      'endpoint',
      'server_name',
      'ca_pem',
      'device',
      'token',
      'ipv4',
      'ipv6',
      'routes',
      'dns',
    };
    if (value.keys.toSet().difference(keys).isNotEmpty ||
        !keys.every(value.containsKey)) {
      throw const FormatException('Invalid profile fields');
    }
    for (final key in [
      'endpoint',
      'server_name',
      'ca_pem',
      'device',
      'token',
      'ipv4',
      'ipv6',
    ]) {
      if (value[key] is! String)
        throw const FormatException('Invalid profile value');
    }
    if (!RegExp(r'^[a-zA-Z0-9_-]{1,64}$').hasMatch(value['device'] as String) ||
        !RegExp(r'^[0-9a-f]{64}$').hasMatch(value['token'] as String) ||
        !(value['ca_pem'] as String).contains('-----BEGIN CERTIFICATE-----') ||
        (value['ca_pem'] as String).length > 32768) {
      throw const FormatException('Invalid credentials');
    }
    final endpoint = value['endpoint'] as String;
    final split = endpoint.lastIndexOf(':');
    if (split < 1) throw const FormatException('Invalid endpoint');
    final host = endpoint
        .substring(0, split)
        .replaceAll('[', '')
        .replaceAll(']', '');
    final port = int.tryParse(endpoint.substring(split + 1));
    if (InternetAddress.tryParse(host) == null ||
        port == null ||
        port < 1 ||
        port > 65535) {
      throw const FormatException('A numeric server endpoint is required');
    }
    final v4 = InternetAddress.tryParse(value['ipv4'] as String);
    final v6 = InternetAddress.tryParse(value['ipv6'] as String);
    if (v4?.type != InternetAddressType.IPv4 ||
        v6?.type != InternetAddressType.IPv6) {
      throw const FormatException('Invalid tunnel addresses');
    }
    final routes = (value['routes'] as List).cast<String>();
    final dns = (value['dns'] as List).cast<String>();
    if (routes.isEmpty || routes.length > 64 || dns.isEmpty || dns.length > 4) {
      throw const FormatException('Invalid routes or DNS');
    }
    bool contains(String cidr, InternetAddress ip) {
      final parts = cidr.split('/');
      final network = InternetAddress.tryParse(parts.first);
      final prefix = parts.length == 2 ? int.tryParse(parts[1]) : null;
      if (network == null ||
          prefix == null ||
          prefix < 0 ||
          prefix > network.rawAddress.length * 8) {
        throw const FormatException('Invalid route');
      }
      if (network.type != ip.type) return false;
      for (var bit = 0; bit < prefix; bit++) {
        final mask = 1 << (7 - bit % 8);
        if (network.rawAddress[bit ~/ 8] & mask !=
            ip.rawAddress[bit ~/ 8] & mask)
          return false;
      }
      return true;
    }

    for (final route in routes) {
      contains(route, v4!);
    }
    for (final server in dns) {
      final ip = InternetAddress.tryParse(server);
      if (ip == null || !routes.any((r) => contains(r, ip))) {
        throw const FormatException('DNS must use a captured route');
      }
    }
    return TunnelProfile._(jsonEncode(value));
  }
}

abstract interface class TunnelAdapter {
  Stream<TunnelState> get states;
  Future<void> connect(TunnelProfile profile);
  Future<void> disconnect();
  void dispose();
}

class NativeTunnelAdapter implements TunnelAdapter {
  static const channel = MethodChannel('org.routeloom/vpn');
  final _states = StreamController<TunnelState>.broadcast();
  Timer? _timer;
  bool _disposed = false;
  @override
  Stream<TunnelState> get states => _states.stream;
  void _emit(TunnelState value) {
    if (!_disposed) _states.add(value);
  }

  @override
  Future<void> connect(TunnelProfile profile) async {
    _emit(TunnelState.connecting);
    try {
      await channel.invokeMethod<void>('connect', profile.json);
      await _poll();
    } catch (_) {
      _emit(TunnelState.failed);
      rethrow;
    }
  }

  Future<void> _poll() async {
    if (_disposed) return;
    try {
      final status = await channel.invokeMethod<String>('status');
      if (_disposed) return;
      final state = TunnelState.values.firstWhere(
        (s) => s.name == status,
        orElse: () => TunnelState.failed,
      );
      _emit(state);
      if (state == TunnelState.connecting ||
          state == TunnelState.connected ||
          state == TunnelState.disconnecting) {
        _timer?.cancel();
        _timer = Timer(const Duration(seconds: 1), _poll);
      }
    } catch (_) {
      _emit(TunnelState.failed);
    }
  }

  @override
  Future<void> disconnect() async {
    _emit(TunnelState.disconnecting);
    await channel.invokeMethod<void>('disconnect');
    await _poll();
  }

  @override
  void dispose() {
    _disposed = true;
    _timer?.cancel();
    _states.close();
  }
}

class DesktopTunnelAdapter implements TunnelAdapter {
  final _states = StreamController<TunnelState>.broadcast();
  Process? _process;
  bool _disposed = false;
  @override
  Stream<TunnelState> get states => _states.stream;
  void _emit(TunnelState value) {
    if (!_disposed) _states.add(value);
  }

  @override
  Future<void> connect(TunnelProfile profile) async {
    if (_process != null) throw StateError('Already running');
    _emit(TunnelState.connecting);
    final suffix = Platform.isWindows ? '.exe' : '';
    final binary =
        Platform.environment['ROUTELOOM_VPN_BINARY'] ??
        File(Platform.resolvedExecutable).parent.path +
            Platform.pathSeparator +
            'routeloom-vpn' +
            suffix;
    try {
      final process = await Process.start(binary, ['connect-stdio']);
      _process = process;
      process.stdin.writeln(profile.json);
      await process.stdin.flush();
      process.stdout
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen((line) {
            if (line == '{"state":"connected"}') _emit(TunnelState.connected);
          });
      // Do not expose child logs or profiles in UI diagnostics.
      process.stderr.drain<void>();
      unawaited(
        process.exitCode.then((code) {
          if (identical(_process, process)) {
            _process = null;
            _emit(code == 0 ? TunnelState.disconnected : TunnelState.failed);
          }
        }),
      );
    } catch (_) {
      _emit(TunnelState.failed);
      rethrow;
    }
  }

  @override
  Future<void> disconnect() async {
    final process = _process;
    if (process == null) return;
    _emit(TunnelState.disconnecting);
    // Graceful stdin shutdown lets the helper roll back routes and DNS on Windows too.
    process.stdin.writeln('disconnect');
    await process.stdin.close();
    await process.exitCode.timeout(const Duration(seconds: 15));
  }

  @override
  void dispose() {
    _disposed = true;
    final process = _process;
    if (process != null) {
      process.stdin.close();
    }
    _states.close();
  }
}

TunnelAdapter createTunnelAdapter() => switch (defaultTargetPlatform) {
  TargetPlatform.windows || TargetPlatform.linux => DesktopTunnelAdapter(),
  _ => NativeTunnelAdapter(),
};
const clientPlatforms = [
  TargetPlatform.windows,
  TargetPlatform.macOS,
  TargetPlatform.linux,
  TargetPlatform.android,
  TargetPlatform.iOS,
];
String platformName(TargetPlatform platform) => switch (platform) {
  TargetPlatform.windows => 'Windows',
  TargetPlatform.macOS => 'macOS',
  TargetPlatform.linux => 'Linux',
  TargetPlatform.android => 'Android',
  TargetPlatform.iOS => 'iOS / iPadOS',
  TargetPlatform.fuchsia => 'Unsupported',
};
