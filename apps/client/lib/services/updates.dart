import 'dart:convert';
import 'dart:io';
import 'package:crypto/crypto.dart' as hashes;
import 'package:cryptography/cryptography.dart';
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';
import 'update_key.dart';

const currentReleaseSequence = 2;
const _assetNames = {
  'windows': 'routeloom-windows-x64.zip',
  'linux': 'routeloom-linux-amd64.deb',
  'android': 'routeloom-android.apk',
  'macos': 'routeloom-apple-signing-kit.zip',
  'ios': 'routeloom-apple-signing-kit.zip',
};

class VerifiedUpdate {
  VerifiedUpdate(this.sequence, this.tag, this.assets);
  final int sequence;
  final String tag;
  final Map<String, dynamic> assets;
  Uri asset(String platform) => Uri.https(
    'github.com',
    '/vvvoip/routeloom/releases/download/' + tag + '/' + _assetNames[platform]!,
  );
  Uri get page =>
      Uri.https('github.com', '/vvvoip/routeloom/releases/tag/' + tag);
}

Future<VerifiedUpdate> verifyUpdate(
  String envelope, {
  String key = updatePublicKey,
  int minimumSequence = currentReleaseSequence,
  DateTime? now,
}) async {
  if (envelope.length > 65536)
    throw const FormatException('Manifest too large');
  final wrapper = jsonDecode(envelope) as Map<String, dynamic>;
  if (wrapper['keyId'] != 'routeloom-2026-01')
    throw const FormatException('Unknown update key');
  final payload = base64Decode(wrapper['payload'] as String);
  final signature = base64Decode(wrapper['signature'] as String);
  if (payload.length > 32768 || signature.length != 64)
    throw const FormatException('Invalid signature');
  final publicKey = SimplePublicKey(
    base64Decode(key),
    type: KeyPairType.ed25519,
  );
  if (!await Ed25519().verify(
    payload,
    signature: Signature(signature, publicKey: publicKey),
  ))
    throw const FormatException('Untrusted update');
  final value = jsonDecode(utf8.decode(payload)) as Map<String, dynamic>;
  final sequence = value['sequence'] as int;
  final tag = value['tag'] as String;
  final published = DateTime.parse(value['published'] as String);
  if (value['schema'] != 1 ||
      sequence < minimumSequence ||
      sequence > 2147483647 ||
      !RegExp(r'^v[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?$').hasMatch(tag) ||
      published.isAfter(
        (now ?? DateTime.now()).toUtc().add(const Duration(days: 1)),
      ))
    throw const FormatException('Invalid release');
  final assets = value['assets'] as Map<String, dynamic>;
  for (final platform in _assetNames.keys) {
    final a = assets[platform] as Map<String, dynamic>;
    if (a['name'] != _assetNames[platform] ||
        a['size'] is! int ||
        a['size'] <= 0 ||
        a['size'] > 262144000 ||
        !RegExp(r'^[0-9a-f]{64}$').hasMatch(a['sha256'] as String))
      throw const FormatException('Invalid update asset');
  }
  return VerifiedUpdate(sequence, tag, assets);
}

class UpdateService {
  static String get platform => Platform.isWindows
      ? 'windows'
      : Platform.isLinux
      ? 'linux'
      : Platform.isAndroid
      ? 'android'
      : Platform.isMacOS
      ? 'macos'
      : 'ios';
  final HttpClient _client = HttpClient()
    ..connectionTimeout = const Duration(seconds: 15);
  Future<List<int>> _bytes(Uri uri, int limit) async {
    final request = await _client.getUrl(uri);
    request.headers.set(HttpHeaders.userAgentHeader, 'RouteLoom updater');
    final response = await request.close().timeout(const Duration(seconds: 20));
    if (response.statusCode != 200) {
      await response.drain<void>();
      throw const HttpException('Update metadata unavailable');
    }
    final bytes = <int>[];
    await for (final chunk in response.timeout(const Duration(seconds: 20))) {
      if (bytes.length + chunk.length > limit)
        throw const FormatException('Response too large');
      bytes.addAll(chunk);
    }
    return bytes;
  }

  Future<VerifiedUpdate?> check(int minimum) async {
    final releases =
        jsonDecode(
              utf8.decode(
                await _bytes(
                  Uri.https(
                    'api.github.com',
                    '/repos/vvvoip/routeloom/releases',
                    {'per_page': '10'},
                  ),
                  1048576,
                ),
              ),
            )
            as List;
    for (final release in releases) {
      if (release['draft'] == true) continue;
      final tag = release['tag_name'] as String;
      if (!RegExp(
        r'^v[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?$',
      ).hasMatch(tag))
        continue;
      if (!(release['assets'] as List).any((a) => a['name'] == 'update.json'))
        continue;
      final raw = utf8.decode(
        await _bytes(
          Uri.https(
            'github.com',
            '/vvvoip/routeloom/releases/download/' + tag + '/update.json',
          ),
          65536,
        ),
      );
      final update = await verifyUpdate(
        raw,
        minimumSequence: minimum > currentReleaseSequence
            ? minimum
            : currentReleaseSequence,
      );
      if (update.tag != tag) throw const FormatException('Release mismatch');
      return update.sequence > currentReleaseSequence ? update : null;
    }
    return null;
  }

  Future<File> download(VerifiedUpdate update) async {
    final asset = update.assets[platform] as Map<String, dynamic>;
    final directory = Directory(
      (await getTemporaryDirectory()).path + Platform.pathSeparator + 'updates',
    );
    await directory.create(recursive: true);
    final file = File(
      directory.path + Platform.pathSeparator + _assetNames[platform]!,
    );
    final partial = File(file.path + '.part');
    final request = await _client.getUrl(update.asset(platform));
    final response = await request.close().timeout(const Duration(seconds: 30));
    if (response.statusCode != 200) {
      await response.drain<void>();
      throw const HttpException('Update download failed');
    }
    final sink = partial.openWrite();
    var count = 0;
    try {
      await for (final chunk in response.timeout(const Duration(seconds: 30))) {
        count += chunk.length;
        if (count > asset['size'])
          throw const FormatException('Update size mismatch');
        sink.add(chunk);
      }
      await sink.flush();
      await sink.close();
      if (count != asset['size'] ||
          (await hashes.sha256.bind(partial.openRead()).first).toString() !=
              asset['sha256'])
        throw const FormatException('Update integrity check failed');
      if (await file.exists()) await file.delete();
      return await partial.rename(file.path);
    } catch (_) {
      await sink.close();
      if (await partial.exists()) await partial.delete();
      rethrow;
    }
  }

  static Future<void> releasePage(VerifiedUpdate update) async {
    if (!await launchUrl(update.page, mode: LaunchMode.externalApplication))
      throw StateError('Cannot open release');
  }

  void close() => _client.close(force: true);
}
