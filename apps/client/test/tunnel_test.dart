import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:routeloom/platforms/tunnel_adapter.dart';

Map<String, dynamic> fixture() => {
  'endpoint': '192.0.2.1:9443',
  'server_name': 'localhost',
  'ca_pem': '-----BEGIN CERTIFICATE-----\ntest-only\n-----END CERTIFICATE-----',
  'device': 'test',
  'token': List.filled(64, 'a').join(),
  'ipv4': '10.77.0.2',
  'ipv6': 'fd77::2',
  'routes': ['0.0.0.0/0', '::/0'],
  'dns': ['1.1.1.1'],
};
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'Linux elevates only the helper and keeps paths as separate arguments',
    () {
      const binary = '/usr/lib/Route Loom/routeloom-vpn';
      expect(desktopTunnelCommand(binary, linux: true), [
        '/usr/bin/pkexec',
        '--disable-internal-agent',
        binary,
        'connect-stdio',
      ]);
      expect(desktopTunnelCommand(binary, linux: false), [
        binary,
        'connect-stdio',
      ]);
    },
  );
  test(
    'profiles reject missing fields, bad identity, routes and uncaptured DNS',
    () {
      expect(TunnelProfile.parse(jsonEncode(fixture())).json, isNotEmpty);
      for (final mutation in [
        (Map<String, dynamic> p) => p.remove('ca_pem'),
        (Map<String, dynamic> p) => p['token'] = 'short',
        (Map<String, dynamic> p) => p['endpoint'] = '10.77.0.1:9443',
        (Map<String, dynamic> p) => p['dns'] = ['192.0.2.1'],
        (Map<String, dynamic> p) => p['endpoint'] = 'unresolved.example:443',
        (Map<String, dynamic> p) => p['routes'] = ['10.0.0.0/8'],
        (Map<String, dynamic> p) => p['routes'] = ['0.0.0.0/33'],
        (Map<String, dynamic> p) => p['unknown'] = 'ignored?',
      ]) {
        final p = fixture();
        mutation(p);
        expect(
          () => TunnelProfile.parse(jsonEncode(p)),
          throwsA(isA<FormatException>()),
        );
      }
    },
  );
  test(
    'native adapter reports connected only after provider status confirms it',
    () async {
      final adapter = NativeTunnelAdapter();
      final states = <TunnelState>[];
      final subscription = adapter.states.listen(states.add);
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, (
        call,
      ) async {
        if (call.method == 'status') return 'connected';
        if (call.method == 'connect') expect(call.arguments, isA<String>());
        return null;
      });
      await adapter.connect(TunnelProfile.parse(jsonEncode(fixture())));
      await Future<void>.delayed(Duration.zero);
      expect(states, [TunnelState.connecting, TunnelState.connected]);
      adapter.dispose();
      await subscription.cancel();
      messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, null);
    },
  );
  test('native permission or TLS errors do not report connected', () async {
    final adapter = NativeTunnelAdapter();
    final states = <TunnelState>[];
    final subscription = adapter.states.listen(states.add);
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, (_) async {
      throw PlatformException(code: 'permission');
    });
    await adapter.connect(TunnelProfile.parse(jsonEncode(fixture())));
    await Future<void>.delayed(Duration.zero);
    expect(states, [TunnelState.connecting, TunnelState.failed]);
    adapter.dispose();
    await subscription.cancel();
    messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, null);
  });
}
