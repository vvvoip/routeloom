import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:routeloom/main.dart';
import 'package:routeloom/platforms/tunnel_adapter.dart';
import 'package:routeloom/services/distribution.dart';
import 'package:routeloom/services/profile_store.dart';
import 'package:routeloom/services/updates.dart';
import 'app_test.dart' show TestVault;
import 'tunnel_test.dart' show fixture;

class NoNetworkClient extends Fake implements HttpClient {
  int requests = 0;
  @override
  set connectionTimeout(Duration? value) {}
  @override
  Future<HttpClientRequest> getUrl(Uri url) async {
    requests++;
    throw StateError('Unexpected network request');
  }

  @override
  void close({bool force = false}) {}
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  if (!appleStoreDistribution) {
    test(
      'Apple Store suite requires the distribution build flag',
      () {},
      skip: 'Run with --dart-define=ROUTELOOM_APPLE_STORE=true',
    );
    return;
  }
  test(
    'store updates cannot contact GitHub, write packages or install',
    () async {
      final client = NoNetworkClient();
      await HttpOverrides.runZoned(() async {
        final updates = UpdateService();
        final release = VerifiedUpdate(999, 'v999.0.0', {});
        var disconnected = false;
        await expectLater(updates.check(0), throwsStateError);
        await expectLater(updates.download(release), throwsStateError);
        await expectLater(UpdateService.releasePage(release), throwsStateError);
        await expectLater(
          updates.install(release, File('nonexistent-test-package'), () async {
            disconnected = true;
          }),
          throwsStateError,
        );
        expect(client.requests, 0);
        expect(disconnected, false);
        updates.close();
      }, createHttpClient: (_) => client);
    },
  );
  for (final platform in [TargetPlatform.iOS, TargetPlatform.macOS]) {
    for (final language in ['en', 'zh']) {
      testWidgets(
        'store disclosure gates connection on $platform in $language',
        (tester) async {
          debugDefaultTargetPlatformOverride = platform;
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
            messenger.setMockMethodCallHandler(
              NativeTunnelAdapter.channel,
              null,
            );
          });
          final store = ProfileStore(vault: TestVault());
          await store.load();
          store.language = language;
          store.automaticUpdates = true;
          await store.import(TunnelProfile.parse(jsonEncode(fixture())));
          await tester.pumpWidget(RouteLoomApp(store: store));
          await tester.pumpAndSettle();
          final connect = find.byKey(const Key('connect-button'));
          await tester.ensureVisible(connect);
          await tester.tap(connect);
          await tester.pumpAndSettle();
          expect(connects, 0);
          expect(
            find.text(language == 'en' ? 'Before you connect' : '连接前请了解'),
            findsOneWidget,
          );
          await tester.tap(find.text(language == 'en' ? 'Cancel' : '取消'));
          await tester.pumpAndSettle();
          expect(connects, 0);
          await tester.tap(connect);
          await tester.pumpAndSettle();
          final accept = find.byKey(const Key('accept-vpn-disclosure'));
          if (publisherPrivacyPolicy == null) {
            expect(tester.widget<FilledButton>(accept).onPressed, isNull);
            expect(connects, 0);
          } else {
            expect(
              find.byKey(const Key('publisher-privacy-policy')),
              findsOneWidget,
            );
            await tester.tap(accept);
            await tester.pumpAndSettle();
            expect(connects, 1);
          }
          // No source ZIP or external update controls in an Apple-managed build.
          expect(
            find.text(language == 'en' ? 'Check for updates' : '检查更新'),
            findsNothing,
          );
          await tester.pumpWidget(const SizedBox());
        },
      );
    }
  }
}
