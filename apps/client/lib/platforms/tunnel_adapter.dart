import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

enum TunnelState {
  unavailable,
  disconnected,
  connecting,
  reconnecting,
  blocked,
  connected,
  disconnecting,
  failed,
}

class TunnelProfile {
  TunnelProfile._(this.json);
  final String json;
  Map<String, dynamic> get data => jsonDecode(json) as Map<String, dynamic>;
  Map<String, dynamic> get options => {
    'mode': 'auto',
    'policy': 'final proxy:default',
    'auto_reconnect': true,
    'kill_switch': false,
    'pac_port': 12789,
    ...((data['options'] as Map<String, dynamic>?) ?? {}),
  };
  TunnelProfile withOptions(Map<String, dynamic> changes) =>
      TunnelProfile.parse(
        jsonEncode({
          ...data,
          'options': {...options, ...changes},
        }),
      );

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
    if (value.keys.toSet().difference({...keys, 'options'}).isNotEmpty ||
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
    final endpointIP = InternetAddress.tryParse(host)!;
    final endpointBytes = endpointIP.rawAddress;
    if ((endpointIP.type == InternetAddressType.IPv4 &&
            endpointBytes[0] == 10 &&
            endpointBytes[1] == 77 &&
            endpointBytes[2] == 0) ||
        (endpointIP.type == InternetAddressType.IPv6 &&
            endpointBytes[0] == 0xfd &&
            endpointBytes[1] == 0x77 &&
            endpointBytes.sublist(2, 8).every((b) => b == 0))) {
      throw const FormatException('Server overlaps the tunnel subnet');
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
      if (ip == null ||
          ip == endpointIP ||
          !routes.any((r) => contains(r, ip))) {
        throw const FormatException('DNS must use a captured route');
      }
    }
    if (value.containsKey('options')) {
      final o = value['options'] as Map<String, dynamic>;
      if (o.keys.toSet().difference({
            'mode',
            'policy',
            'auto_reconnect',
            'kill_switch',
            'pac_port',
          }).isNotEmpty ||
          (o.containsKey('mode') &&
              !['auto', 'proxy', 'direct'].contains(o['mode'])) ||
          (o.containsKey('policy') &&
              (o['policy'] is! String ||
                  (o['policy'] as String).length > 16384)) ||
          (o.containsKey('auto_reconnect') && o['auto_reconnect'] is! bool) ||
          (o.containsKey('kill_switch') && o['kill_switch'] is! bool) ||
          (o.containsKey('pac_port') &&
              (o['pac_port'] is! int ||
                  o['pac_port'] < 0 ||
                  o['pac_port'] > 65535))) {
        throw const FormatException('Invalid options');
      }
    }
    return TunnelProfile._(jsonEncode(value));
  }
}

abstract interface class TunnelAdapter {
  Stream<TunnelState> get states;
  Future<void> connect(TunnelProfile profile);
  Future<void> disconnect();
  Future<void> refresh();
  Future<Map<String, dynamic>?> snapshot();
  Future<void> monitorCommand(String command);
  void dispose();
}

class NativeTunnelAdapter implements TunnelAdapter {
  static const channel = MethodChannel('org.routeloom/vpn');
  final _states = StreamController<TunnelState>.broadcast();
  Timer? _timer;
  bool _disposed = false;
  bool _starting = false;
  int _operation = 0;
  int _pollSequence = 0;
  @override
  Stream<TunnelState> get states => _states.stream;
  void _emit(TunnelState value) {
    if (!_disposed) _states.add(value);
  }

  @override
  Future<void> connect(TunnelProfile profile) async {
    final operation = ++_operation;
    _timer?.cancel();
    _starting = true;
    _emit(TunnelState.connecting);
    try {
      await channel.invokeMethod<void>('connect', profile.json);
      if (operation == _operation) {
        _starting = false;
        await _poll(operation);
      }
    } catch (_) {
      if (operation == _operation) {
        _starting = false;
        _emit(TunnelState.failed);
      }
    }
  }

  @override
  Future<void> refresh() async {
    if (!_starting) await _poll(_operation);
  }

  @override
  Future<Map<String, dynamic>?> snapshot() async {
    final raw = await channel.invokeMethod<String>('snapshot');
    if (raw == null || raw.isEmpty) return null;
    if (raw.length > 1048576) throw const FormatException('Snapshot too large');
    return jsonDecode(raw) as Map<String, dynamic>;
  }

  @override
  Future<void> monitorCommand(String command) async {
    if (!['clear', 'diagnose'].contains(command))
      throw ArgumentError('Unknown command');
    if (await channel.invokeMethod<bool>('monitorCommand', command) != true)
      throw StateError('Monitor unavailable');
  }

  Future<void> _poll(int operation) async {
    if (_disposed || operation != _operation) return;
    _timer?.cancel();
    final sequence = ++_pollSequence;
    bool current() =>
        !_disposed && operation == _operation && sequence == _pollSequence;
    var supported = true;
    var interval = const Duration(seconds: 5);
    try {
      final status = await channel.invokeMethod<String>('status');
      if (!current()) return;
      final state = TunnelState.values.firstWhere(
        (s) => s.name == status,
        orElse: () => TunnelState.failed,
      );
      _emit(state);
      if (state == TunnelState.reconnecting ||
          state == TunnelState.blocked ||
          state == TunnelState.connecting ||
          state == TunnelState.connected ||
          state == TunnelState.disconnecting) {
        interval = const Duration(seconds: 1);
      }
    } on MissingPluginException {
      supported = false;
      if (current()) _emit(TunnelState.unavailable);
    } catch (_) {
      if (current()) _emit(TunnelState.failed);
    } finally {
      // The OS can reconnect on demand after a disconnect or temporary IPC failure.
      // Keep observing it without starting a tunnel or changing protection settings.
      if (current() && supported) {
        _timer = Timer(interval, () => _poll(operation));
      }
    }
  }

  @override
  Future<void> disconnect() async {
    final operation = ++_operation;
    _starting = false;
    _timer?.cancel();
    _emit(TunnelState.disconnecting);
    try {
      await channel.invokeMethod<void>('disconnect');
      await _poll(operation);
    } catch (_) {
      if (operation == _operation) _emit(TunnelState.failed);
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _timer?.cancel();
    _states.close();
  }
}

List<String> desktopTunnelCommand(String binary, {required bool linux}) => linux
    ? ['/usr/bin/pkexec', '--disable-internal-agent', binary, 'connect-stdio']
    : [binary, 'connect-stdio'];

class DesktopTunnelAdapter implements TunnelAdapter {
  final _states = StreamController<TunnelState>.broadcast();
  Process? _process;
  Completer<Map<String, dynamic>?>? _snapshot;
  bool _disposed = false;
  int _operation = 0;
  TunnelState _state = TunnelState.disconnected;
  @override
  Stream<TunnelState> get states => _states.stream;
  void _emit(TunnelState value) {
    _state = value;
    if (!_disposed) _states.add(value);
  }

  @override
  Future<void> connect(TunnelProfile profile) async {
    if (_process != null && _state == TunnelState.blocked) await disconnect();
    if (_process != null) throw StateError('Already running');
    final operation = ++_operation;
    _emit(TunnelState.connecting);
    final suffix = Platform.isWindows ? '.exe' : '';
    final binary =
        Platform.environment['ROUTELOOM_VPN_BINARY'] ??
        File(Platform.resolvedExecutable).parent.path +
            Platform.pathSeparator +
            'routeloom-vpn' +
            suffix;
    try {
      // Only the helper is elevated on Linux; the UI remains the logged-in user.
      final command = desktopTunnelCommand(binary, linux: Platform.isLinux);
      final process = await Process.start(command.first, command.sublist(1));
      if (_disposed || operation != _operation) {
        process.stdout.drain<void>();
        process.stderr.drain<void>();
        await process.stdin.close();
        return;
      }
      _process = process;
      process.stdin.writeln(profile.json);
      await process.stdin.flush();
      process.stdout
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen((line) {
            try {
              final message = jsonDecode(line) as Map<String, dynamic>;
              if (message['telemetry'] is Map<String, dynamic>) {
                final pending = _snapshot;
                _snapshot = null;
                if (pending != null && !pending.isCompleted)
                  pending.complete(
                    message['telemetry'] as Map<String, dynamic>,
                  );
                return;
              }
              final status = message['state'];
              final state = TunnelState.values
                  .where((s) => s.name == status)
                  .firstOrNull;
              if (state != null) _emit(state);
            } catch (_) {
              /* Ignore non-status output. */
            }
          });
      // Do not expose child logs or profiles in UI diagnostics.
      process.stderr.drain<void>();
      unawaited(
        process.exitCode.then((code) {
          if (identical(_process, process)) {
            _process = null;
            final pending = _snapshot;
            _snapshot = null;
            if (pending != null && !pending.isCompleted) pending.complete(null);
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
    ++_operation;
    final process = _process;
    if (process == null) {
      _emit(TunnelState.disconnected);
      return;
    }
    _emit(TunnelState.disconnecting);
    // Graceful stdin shutdown lets the helper roll back routes and DNS on Windows too.
    process.stdin.writeln('disconnect');
    await process.stdin.close();
    await process.exitCode.timeout(const Duration(seconds: 15));
  }

  @override
  Future<void> refresh() async {
    _emit(_state);
  }

  @override
  Future<Map<String, dynamic>?> snapshot() async {
    if (_process == null) return null;
    if (_snapshot != null) return _snapshot!.future;
    final pending = Completer<Map<String, dynamic>?>();
    _snapshot = pending;
    try {
      await monitorCommand('snapshot');
      return await pending.future.timeout(const Duration(seconds: 3));
    } finally {
      if (identical(_snapshot, pending)) {
        _snapshot = null;
        if (!pending.isCompleted) pending.complete(null);
      }
    }
  }

  @override
  Future<void> monitorCommand(String command) async {
    if (!['clear', 'diagnose', 'snapshot'].contains(command))
      throw ArgumentError('Unknown command');
    final process = _process;
    if (process == null) throw StateError('Monitor unavailable');
    process.stdin.writeln(command);
    await process.stdin.flush();
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
