import 'dart:async';
import 'dart:convert';
import 'tunnel_test.dart' show fixture;
import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:routeloom/main.dart';
import 'package:routeloom/services/profile_store.dart';
import 'package:routeloom/platforms/tunnel_adapter.dart';

void main() {
  testWidgets('cannot connect until a routing-mode save finishes', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var connects = 0;
    messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, (
      call,
    ) async {
      if (call.method == 'status') return 'disconnected';
      if (call.method == 'connect') connects++;
      return null;
    });
    addTearDown(() {
      debugDefaultTargetPlatformOverride = null;
      messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, null);
    });
    final vault = DelayedVault();
    final store = ProfileStore(vault: vault);
    await store.load();
    await store.import(TunnelProfile.parse(jsonEncode(fixture())));
    await tester.pumpWidget(RouteLoomApp(store: store));
    await tester.pumpAndSettle();
    vault.pending = Completer<void>();
    final picker = tester.widget<DropdownButton<String>>(
      find.byKey(const Key('routing-mode')),
    );
    picker.onChanged!('direct');
    await tester.pump();
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('connect-button')))
          .onPressed,
      isNull,
    );
    expect(connects, 0);
    vault.pending!.completeError(StateError('Storage unavailable'));
    await tester.pumpAndSettle();
    expect(store.active!.profile.options['mode'], 'auto');
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('connect-button')))
          .onPressed,
      isNotNull,
    );
    expect(connects, 0);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'reopened mobile app can disconnect an existing VPN without reimport',
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      var connected = true;
      messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, (
        call,
      ) async {
        if (call.method == 'disconnect') {
          connected = false;
          return null;
        }
        if (call.method == 'status')
          return connected ? 'connected' : 'disconnected';
        throw PlatformException(code: 'unexpected');
      });
      addTearDown(() {
        debugDefaultTargetPlatformOverride = null;
        messenger.setMockMethodCallHandler(NativeTunnelAdapter.channel, null);
      });
      await tester.pumpWidget(
        RouteLoomApp(store: ProfileStore(vault: TestVault())),
      );
      await tester.pumpAndSettle();
      expect(find.text('Connected'), findsOneWidget);
      final button = tester.widget<FilledButton>(
        find.byKey(const Key('connect-button')),
      );
      expect(button.onPressed, isNotNull);
      await tester.tap(find.byKey(const Key('connect-button')));
      await tester.pumpAndSettle();
      expect(find.text('Not connected'), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('connect requires an imported profile', (tester) async {
    await tester.pumpWidget(
      RouteLoomApp(store: ProfileStore(vault: TestVault())),
    );
    await tester.pumpAndSettle();
    expect(find.text('Not connected'), findsOneWidget);
    final button = tester.widget<FilledButton>(
      find.byKey(const Key('connect-button')),
    );
    expect(button.onPressed, isNull);
    expect(find.text('Developer preview'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets('language can switch to Chinese and back', (tester) async {
    await tester.pumpWidget(
      RouteLoomApp(store: ProfileStore(vault: TestVault())),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('language-selector')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('简体中文').last);
    await tester.pumpAndSettle();
    expect(find.text('设置'), findsOneWidget);
    expect(find.text('隐私设计'), findsOneWidget);
    await tester.tap(find.byKey(const Key('language-selector')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('English').last);
    await tester.pumpAndSettle();
    expect(find.text('Privacy by design'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
  for (final platform in clientPlatforms) {
    testWidgets('interface renders on ' + platformName(platform), (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = platform;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      await tester.pumpWidget(
        RouteLoomApp(store: ProfileStore(vault: TestVault())),
      );
      await tester.pumpAndSettle();
      expect(find.text('RouteLoom'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    });
  }
}

class TestVault implements Vault {
  String raw = '{"version":1,"profiles":[],"automaticUpdates":false}';
  @override
  Future<String?> read() async => raw;
  @override
  Future<void> write(String value) async {
    raw = value;
  }
}

class DelayedVault extends TestVault {
  Completer<void>? pending;
  @override
  Future<void> write(String value) async {
    if (pending != null) await pending!.future;
    await super.write(value);
  }
}
