import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cryptography/cryptography.dart';
import 'package:routeloom/services/profile_store.dart';
import 'package:routeloom/services/updates.dart';
import 'package:routeloom/services/routing_presets.dart';
import 'package:routeloom/widgets/features.dart';
import 'traffic_test.dart' show host;
import 'package:routeloom/platforms/tunnel_adapter.dart';
import 'tunnel_test.dart' show fixture;

class MemoryVault implements Vault {
  String? value;
  bool fail = false;
  @override
  Future<String?> read() async => value;
  @override
  Future<void> write(String raw) async {
    if (fail) throw StateError('keyring locked');
    value = raw;
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(rootBundle.clear);
  for (final locale in ['en', 'zh']) {
    testWidgets(
      'preset replacement is explicit and disabled during capture in $locale',
      (tester) async {
        final store = ProfileStore(vault: MemoryVault());
        await store.load();
        store.automaticUpdates = false;
        final original = TunnelProfile.parse(jsonEncode(fixture()))
            .withOptions({
              'policy': 'domain blocked.test block\nfinal proxy:default',
              'mode': 'proxy',
            });
        await store.import(original);
        Widget panel(bool active) => host(
          FeaturePanel(
            store: store,
            active: active,
            onChanged: () {},
            state: active ? TunnelState.connected : TunnelState.disconnected,
            disconnect: () async {},
          ),
          locale,
        );
        await tester.pumpWidget(panel(false));
        await tester.pumpAndSettle();
        final button = find.byKey(const Key('china-lan-preset'));
        await tester.ensureVisible(button);
        await tester.tap(button);
        await tester.pumpAndSettle();
        await tester.tap(find.text(locale == 'en' ? 'Cancel' : '取消'));
        await tester.pumpAndSettle();
        expect(store.active!.profile.json, original.json);
        await tester.tap(button);
        await tester.pumpAndSettle();
        await tester.tap(find.text(locale == 'en' ? 'Apply preset' : '应用预设'));
        await tester.pumpAndSettle();
        expect(store.active!.profile.options['mode'], 'auto');
        expect(
          store.active!.profile.options['policy'],
          await RoutingPresets.chinaAndLan(),
        );
        await tester.pumpWidget(panel(true));
        await tester.pumpAndSettle();
        expect(tester.widget<OutlinedButton>(button).onPressed, isNull);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
      },
    );
  }
  test(
    'China preset persists without changing credentials or protection',
    () async {
      final vault = MemoryVault();
      final store = ProfileStore(vault: vault);
      await store.load();
      final profile = TunnelProfile.parse(
        jsonEncode(fixture()),
      ).withOptions({'kill_switch': true, 'mode': 'proxy'});
      await store.import(profile);
      final policy = await RoutingPresets.chinaAndLan();
      expect(utf8.encode(policy).length, lessThanOrEqualTo(16384));
      await store.updateActive(
        profile.withOptions({'policy': policy, 'mode': 'auto'}),
      );
      final restored = ProfileStore(vault: vault);
      await restored.load();
      expect(restored.active!.profile.options['policy'], policy);
      expect(restored.active!.profile.options['mode'], 'auto');
      expect(restored.active!.profile.options['kill_switch'], true);
      final before = profile.data;
      final after = restored.active!.profile.data;
      before.remove('options');
      after.remove('options');
      expect(after, before);
    },
  );
  test('profiles, selection, options and language survive restart', () async {
    final vault = MemoryVault();
    final store = ProfileStore(vault: vault);
    await store.load();
    final profile = TunnelProfile.parse(jsonEncode(fixture()));
    await store.import(profile);
    await store.updateActive(
      profile.withOptions({
        'policy': 'domain blocked.test block\nfinal proxy:default',
        'kill_switch': true,
      }),
      name: 'Work',
    );
    store.language = 'zh';
    await store.save();
    final restored = ProfileStore(vault: vault);
    await restored.load();
    expect(restored.active!.name, 'Work');
    expect(restored.language, 'zh');
    expect(restored.active!.profile.options['kill_switch'], true);
    await restored.import(profile);
    expect(restored.profiles.length, 1);
    await restored.delete(restored.active!.id);
    expect(restored.active, isNull);
  });
  test('failed secure writes do not falsely report a saved profile', () async {
    final vault = MemoryVault();
    final store = ProfileStore(vault: vault);
    await store.load();
    vault.fail = true;
    await expectLater(
      store.import(TunnelProfile.parse(jsonEncode(fixture()))),
      throwsStateError,
    );
    expect(store.profiles, isEmpty);
    expect(vault.value, isNull);
  });
  test(
    'signed updates reject tampering, rollback and path substitution',
    () async {
      final algorithm = Ed25519();
      final key = await algorithm.newKeyPair();
      final public = base64Encode((await key.extractPublicKey()).bytes);
      final manifest = <String, dynamic>{
        'schema': 1,
        'sequence': 4,
        'tag': 'v0.2.1-alpha.1',
        'published': '2026-11-13T00:00:00Z',
        'assets': {
          for (final e in {
            'windows': 'routeloom-windows-x64.zip',
            'linux': 'routeloom-linux-amd64.deb',
            'android': 'routeloom-android.apk',
            'macos': 'routeloom-apple-signing-kit.zip',
            'ios': 'routeloom-apple-signing-kit.zip',
          }.entries)
            e.key: {
              'name': e.value,
              'size': 123,
              'sha256': List.filled(64, 'a').join(),
            },
        },
      };
      Future<String> envelope() async {
        final bytes = utf8.encode(jsonEncode(manifest));
        final signature = await algorithm.sign(bytes, keyPair: key);
        return jsonEncode({
          'keyId': 'routeloom-2026-01',
          'payload': base64Encode(bytes),
          'signature': base64Encode(signature.bytes),
        });
      }

      final signed = await envelope();
      expect(
        (await verifyUpdate(
          signed,
          key: public,
          now: DateTime.utc(2026, 11, 13),
        )).sequence,
        4,
      );
      await expectLater(
        verifyUpdate(signed, key: public, minimumSequence: 5),
        throwsFormatException,
      );
      final damaged = jsonDecode(signed) as Map<String, dynamic>;
      damaged['payload'] = base64Encode(
        utf8.encode(jsonEncode({...manifest, 'sequence': 99})),
      );
      await expectLater(
        verifyUpdate(jsonEncode(damaged), key: public),
        throwsFormatException,
      );
      ((manifest['assets'] as Map)['windows'] as Map)['name'] =
          '../../evil.exe';
      await expectLater(
        verifyUpdate(await envelope(), key: public),
        throwsFormatException,
      );
    },
  );
}
