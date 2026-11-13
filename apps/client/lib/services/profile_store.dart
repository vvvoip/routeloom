import 'dart:convert';
import 'dart:math';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import '../platforms/tunnel_adapter.dart';

abstract interface class Vault {
  Future<String?> read();
  Future<void> write(String value);
}

class SystemVault implements Vault {
  const SystemVault();
  static const _storage = FlutterSecureStorage();
  @override
  Future<String?> read() => _storage.read(key: 'routeloom.profiles.v1');
  @override
  Future<void> write(String value) =>
      _storage.write(key: 'routeloom.profiles.v1', value: value);
}

class SavedProfile {
  SavedProfile(this.id, this.name, this.profile);
  final String id, name;
  final TunnelProfile profile;
  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'profile': jsonDecode(profile.json),
  };
}

class ProfileStore {
  ProfileStore({this.vault = const SystemVault()});
  final Vault vault;
  List<SavedProfile> profiles = [];
  String? selected;
  String language = 'system';
  bool automaticUpdates = true, available = false;
  int highestUpdate = 0;
  DateTime? lastUpdateCheck;
  Future<void> _pending = Future.value();
  SavedProfile? get active =>
      profiles.where((p) => p.id == selected).firstOrNull;
  Future<void> load() async {
    final raw = await vault.read();
    if (raw != null) {
      if (raw.length > 1500000) throw const FormatException('Vault too large');
      final data = jsonDecode(raw) as Map<String, dynamic>;
      if (data['version'] != 1) throw const FormatException('Vault version');
      final rows = data['profiles'] as List;
      if (rows.length > 20) throw const FormatException('Too many profiles');
      profiles = rows
          .map(
            (p) => SavedProfile(
              p['id'] as String,
              p['name'] as String,
              TunnelProfile.parse(jsonEncode(p['profile'])),
            ),
          )
          .toList();
      selected = data['selected'] as String?;
      language = ['en', 'zh', 'system'].contains(data['language'])
          ? data['language'] as String
          : 'system';
      automaticUpdates = data['automaticUpdates'] != false;
      highestUpdate = data['highestUpdate'] as int? ?? 0;
      lastUpdateCheck = DateTime.tryParse(
        data['lastUpdateCheck'] as String? ?? '',
      );
    }
    available = true;
  }

  Future<void> save() {
    if (!available)
      return Future.error(StateError('Secure storage unavailable'));
    final raw = jsonEncode({
      'version': 1,
      'profiles': profiles.map((p) => p.toJson()).toList(),
      'selected': selected,
      'language': language,
      'automaticUpdates': automaticUpdates,
      'highestUpdate': highestUpdate,
      'lastUpdateCheck': lastUpdateCheck?.toUtc().toIso8601String(),
    });
    final result = _pending.catchError((_) {}).then((_) => vault.write(raw));
    _pending = result;
    return result;
  }

  Future<void> import(TunnelProfile profile) async {
    final data = jsonDecode(profile.json) as Map<String, dynamic>;
    final old = profiles.where((p) {
      final value = jsonDecode(p.profile.json) as Map<String, dynamic>;
      return value['device'] == data['device'] &&
          value['endpoint'] == data['endpoint'];
    }).firstOrNull;
    if (old == null && profiles.length >= 20) throw StateError('Profile limit');
    final id =
        old?.id ??
        List.generate(
          16,
          (_) => Random.secure().nextInt(256).toRadixString(16).padLeft(2, '0'),
        ).join();
    final next = SavedProfile(
      id,
      old?.name ?? data['device'] as String,
      profile,
    );
    final previous = [...profiles];
    final previousSelected = selected;
    profiles = [...profiles.where((p) => p.id != id), next];
    selected = id;
    try {
      await save();
    } catch (_) {
      profiles = previous;
      selected = previousSelected;
      rethrow;
    }
  }

  Future<void> updateActive(TunnelProfile profile, {String? name}) async {
    final active = this.active;
    if (active == null) return;
    if (name != null && (name.trim().isEmpty || name.length > 64))
      throw const FormatException('Invalid name');
    final previous = profiles;
    profiles = profiles
        .map(
          (p) => p.id == active.id
              ? SavedProfile(p.id, name?.trim() ?? p.name, profile)
              : p,
        )
        .toList();
    try {
      await save();
    } catch (_) {
      profiles = previous;
      rethrow;
    }
  }

  Future<void> delete(String id) async {
    final previous = [...profiles];
    final previousSelected = selected;
    profiles = profiles.where((p) => p.id != id).toList();
    if (selected == id) selected = profiles.firstOrNull?.id;
    try {
      await save();
    } catch (_) {
      profiles = previous;
      selected = previousSelected;
      rethrow;
    }
  }
}
