import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:cryptography/cryptography.dart';
import 'package:routeloom/services/profile_store.dart';
import 'package:routeloom/services/updates.dart';
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
        'sequence': 3,
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
        3,
      );
      await expectLater(
        verifyUpdate(signed, key: public, minimumSequence: 4),
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
