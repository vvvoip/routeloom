import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:routeloom/l10n/generated/app_localizations.dart';
import 'package:routeloom/platforms/tunnel_adapter.dart';
import 'package:routeloom/services/traffic.dart';
import 'package:routeloom/widgets/traffic.dart';
import 'tunnel_test.dart' show fixture;

Map<String, dynamic> sample() => {
  'schema': 1,
  'session': 123,
  'mode': 'auto',
  'active': 1,
  'upload': 2048,
  'download': 8192,
  'connections': [
    {
      'id': 1,
      'host': 'google.com',
      'source': '10.77.0.2:40000',
      'protocol': 'tcp',
      'port': 443,
      'address': '142.250.1.1:443',
      'route': 'proxy',
      'rule_line': 2,
      'rule': 'final proxy:default',
      'status': 'active',
      'upload': 2048,
      'download': 8192,
      'duration_ms': 5000,
      'connect_ms': 82,
    },
    {
      'id': 2,
      'host': 'dns.example',
      'source': '10.77.0.2:40001',
      'protocol': 'udp',
      'port': 443,
      'address': '192.0.2.20:443',
      'route': 'direct',
      'status': 'closed',
      'upload': 100,
      'download': 200,
    },
  ],
  'diagnostic': {
    'status': 'success',
    'mode': 'auto',
    'route': 'direct',
    'exit_ip': '203.0.113.10',
    'server_ip': '203.0.113.10',
    'matches_server': true,
    'latency_ms': 245,
    'checked_at': 1794567600000,
  },
};

class FakeAdapter implements TunnelAdapter {
  final commands = <String>[];
  @override
  Stream<TunnelState> get states => const Stream.empty();
  @override
  Future<void> connect(TunnelProfile p) async {}
  @override
  Future<void> disconnect() async {}
  @override
  Future<void> refresh() async {}
  @override
  Future<Map<String, dynamic>?> snapshot() async => sample();
  @override
  Future<void> monitorCommand(String command) async {
    commands.add(command);
  }

  @override
  void dispose() {}
}

Widget host(Widget child, String locale) => MaterialApp(
  locale: Locale(locale),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: ListView(padding: const EdgeInsets.all(16), children: [child]),
  ),
);
void main() {
  test('connection filters use actual protocol and route', () {
    final s = TrafficSnapshot(sample());
    expect(
      s.connections
          .where((r) => r.matches('GOOGLE', 'tcp', 'proxy', true))
          .length,
      1,
    );
    expect(
      s.connections.where((r) => r.matches('', 'udp', 'direct', true)),
      isEmpty,
    );
    expect(
      s.connections
          .where((r) => r.matches('192.0.2', 'udp', 'direct', false))
          .length,
      1,
    );
    expect(
      s.diagnostic['route'],
      'direct',
    ); // Equal IPs do not override engine evidence.
  });
  test('old profiles default to auto and invalid modes are rejected', () {
    final p = TunnelProfile.parse(jsonEncode(fixture()));
    expect(p.options['mode'], 'auto');
    expect(p.withOptions({'mode': 'direct'}).options['mode'], 'direct');
    expect(() => p.withOptions({'mode': 'random'}), throwsFormatException);
  });
  for (final locale in ['en', 'zh']) {
    testWidgets('dashboard works on narrow screens in ' + locale, (
      tester,
    ) async {
      tester.view.physicalSize = const Size(390, 844);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final adapter = FakeAdapter();
      await tester.pumpWidget(
        host(
          TrafficPanel(adapter: adapter, active: true, connected: true),
          locale,
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.text(locale == 'en' ? 'HTTPS test passed' : 'HTTPS 测试通过'),
        findsOneWidget,
      );
      expect(
        find.text(
          (locale == 'en' ? 'Route used by this test' : '本次测试实际路径') +
              ': ' +
              (locale == 'en' ? 'Direct' : '直连'),
        ),
        findsOneWidget,
      );
      await tester.ensureVisible(find.byKey(const Key('flow-search')));
      await tester.enterText(find.byKey(const Key('flow-search')), 'google');
      await tester.pumpAndSettle();
      expect(find.text('google.com:443'), findsOneWidget);
      expect(find.text('dns.example:443'), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    });
  }
  testWidgets('mode picker cannot change a connected session', (tester) async {
    var changed = false;
    await tester.pumpWidget(
      host(
        ModePicker(
          mode: 'auto',
          enabled: false,
          onChanged: (_) => changed = true,
        ),
        'en',
      ),
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<DropdownButton<String>>(find.byKey(const Key('routing-mode')))
          .onChanged,
      isNull,
    );
    expect(changed, false);
    await tester.pumpWidget(const SizedBox());
  });
}
