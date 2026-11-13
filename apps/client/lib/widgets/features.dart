import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../l10n/generated/app_localizations.dart';
import '../services/profile_store.dart';
import '../services/routing_presets.dart';
import '../services/protection.dart';
import '../services/updates.dart';
import '../platforms/tunnel_adapter.dart';

class FeaturePanel extends StatefulWidget {
  const FeaturePanel({
    super.key,
    required this.store,
    required this.active,
    required this.onChanged,
    required this.state,
    required this.disconnect,
  });
  final ProfileStore store;
  final bool active;
  final VoidCallback onChanged;
  final TunnelState state;
  final Future<void> Function() disconnect;
  @override
  State<FeaturePanel> createState() => _FeaturePanelState();
}

class _FeaturePanelState extends State<FeaturePanel> {
  final _updates = UpdateService();
  Timer? _timer;
  bool _busy = false, _checking = false;
  String? _updateMessage;
  VerifiedUpdate? _update;
  File? _download;
  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(hours: 6), (_) => _automatic());
    WidgetsBinding.instance.addPostFrameCallback((_) => _automatic());
  }

  @override
  void didUpdateWidget(covariant FeaturePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _automatic();
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    _updates.close();
    super.dispose();
  }

  void _error(String message) {
    if (mounted)
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _run(Future<void> Function() action) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await action();
      widget.onChanged();
    } catch (_) {
      if (mounted) _error(AppLocalizations.of(context).operationFailed);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _automatic() async {
    final store = widget.store;
    if (!store.available || !store.automaticUpdates) return;
    if (store.lastUpdateCheck != null &&
        DateTime.now().difference(store.lastUpdateCheck!) <
            const Duration(hours: 6))
      return;
    await _check();
  }

  Future<void> _check() async {
    if (_checking) return;
    setState(() => _checking = true);
    final t = AppLocalizations.of(context);
    try {
      final update = await _updates.check(widget.store.highestUpdate);
      widget.store.lastUpdateCheck = DateTime.now();
      if (update != null) widget.store.highestUpdate = update.sequence;
      if (widget.store.available) await widget.store.save();
      if (!mounted) return;
      setState(() {
        _update = update;
        _updateMessage = update == null ? t.upToDate : t.updateAvailable;
      });
      if (update != null && !Platform.isIOS && !Platform.isMacOS) {
        final file = await _updates.download(update);
        if (mounted) setState(() => _download = file);
      }
    } catch (_) {
      if (mounted) setState(() => _updateMessage = t.updateFailed);
    } finally {
      if (mounted) setState(() => _checking = false);
    }
  }

  Future<void> _edit() async {
    final saved = widget.store.active;
    if (saved == null) return;
    final t = AppLocalizations.of(context);
    final name = TextEditingController(text: saved.name);
    final rules = TextEditingController(
      text: saved.profile.options['policy'] as String,
    );
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.editProfile),
        content: SizedBox(
          width: 600,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: name,
                  maxLength: 64,
                  decoration: InputDecoration(labelText: t.profileName),
                ),
                TextField(
                  controller: rules,
                  minLines: 8,
                  maxLines: 16,
                  maxLength: 16384,
                  decoration: InputDecoration(
                    labelText: t.routePolicy,
                    helperText: t.rulesExample,
                  ),
                ),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(t.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(t.save),
          ),
        ],
      ),
    );
    if (accepted == true)
      await _run(
        () => widget.store.updateActive(
          saved.profile.withOptions({'policy': rules.text}),
          name: name.text,
        ),
      );
    name.dispose();
    rules.dispose();
  }

  Future<void> _applyChinaPreset() async {
    if (widget.active || _busy) return;
    final saved = widget.store.active;
    if (saved == null) return;
    await _run(() async {
      final policy = await RoutingPresets.chinaAndLan();
      if (!mounted || widget.active) return;
      final t = AppLocalizations.of(context);
      final previous = saved.profile.options['policy'] as String;
      if (previous.trim() != 'final proxy:default' && previous != policy) {
        final accepted = await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(t.chinaPreset),
            content: Text(t.replaceRulesWarning),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: Text(t.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: Text(t.applyPreset),
              ),
            ],
          ),
        );
        if (accepted != true) return;
      }
      if (!mounted || widget.active || widget.store.active?.id != saved.id)
        return;
      await widget.store.updateActive(
        saved.profile.withOptions({'policy': policy, 'mode': 'auto'}),
      );
      if (mounted) _error(t.presetApplied);
    });
  }

  Future<void> _unlock() async {
    final t = AppLocalizations.of(context);
    final yes = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.unlockInternet),
        content: Text(t.unlockWarning),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(t.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(t.unlockInternet),
          ),
        ],
      ),
    );
    if (yes == true)
      await _run(() async {
        await widget.disconnect();
        await Protection.unlock();
      });
  }

  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    final store = widget.store;
    final active = store.active;
    final options = active?.profile.options;
    return Column(
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  t.savedProfiles,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                if (!store.available) Text(t.storageUnavailable),
                if (store.profiles.isEmpty) Text(t.noProfiles),
                if (store.profiles.isNotEmpty)
                  DropdownButton<String>(
                    isExpanded: true,
                    value: store.selected,
                    items: store.profiles
                        .map(
                          (p) => DropdownMenuItem(
                            value: p.id,
                            child: Text(p.name),
                          ),
                        )
                        .toList(),
                    onChanged: widget.active || _busy
                        ? null
                        : (id) => _run(() async {
                            await store.choose(id);
                          }),
                  ),
                if (active != null) ...[
                  Wrap(
                    spacing: 12,
                    children: [
                      TextButton.icon(
                        onPressed: widget.active || _busy ? null : _edit,
                        icon: const Icon(Icons.edit),
                        label: Text(t.editProfile),
                      ),
                      TextButton.icon(
                        onPressed: widget.active || _busy
                            ? null
                            : () => _run(() async {
                                await Protection.forgetProfile(active.profile);
                                await store.delete(active.id);
                              }),
                        icon: const Icon(Icons.delete_outline),
                        label: Text(t.deleteProfile),
                      ),
                    ],
                  ),
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: Text(t.autoReconnect),
                    subtitle: Text(t.reconnectDescription),
                    value: options!['auto_reconnect'] as bool,
                    onChanged: widget.active || _busy
                        ? null
                        : (v) => _run(
                            () => store.updateActive(
                              active.profile.withOptions({'auto_reconnect': v}),
                            ),
                          ),
                  ),
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: Text(t.killSwitch),
                    subtitle: Text(
                      Platform.isAndroid ? t.androidGuard : t.guardDescription,
                    ),
                    value: options['kill_switch'] as bool,
                    onChanged: widget.active || _busy
                        ? null
                        : (v) => _run(
                            () => store.updateActive(
                              active.profile.withOptions({'kill_switch': v}),
                            ),
                          ),
                  ),
                  Text(t.routePolicyDescription),
                  const SizedBox(height: 12),
                  Text(t.chinaPresetDescription),
                  OutlinedButton.icon(
                    key: const Key('china-lan-preset'),
                    onPressed: widget.active || _busy
                        ? null
                        : _applyChinaPreset,
                    icon: const Icon(Icons.alt_route),
                    label: Text(t.chinaPreset),
                  ),
                  const SizedBox(height: 8),
                  Text(t.pacDescription),
                  SelectableText(
                    'http://127.0.0.1:' +
                        options['pac_port'].toString() +
                        '/proxy.pac',
                  ),
                  TextButton(
                    onPressed: () => Clipboard.setData(
                      ClipboardData(
                        text:
                            'http://127.0.0.1:' +
                            options['pac_port'].toString() +
                            '/proxy.pac',
                      ),
                    ),
                    child: Text(t.copyPac),
                  ),
                ],
                if (Platform.isAndroid)
                  FutureBuilder<bool>(
                    future: Protection.nativeStatus(),
                    builder: (context, snapshot) => Text(
                      snapshot.data == true
                          ? t.guardStatusOn
                          : t.guardStatusOff,
                    ),
                  ),
                if (Platform.isAndroid)
                  TextButton(
                    onPressed: () => _run(Protection.settings),
                    child: Text(t.openVpnSettings),
                  ),
                OutlinedButton(
                  onPressed: _busy ? null : _unlock,
                  child: Text(t.unlockInternet),
                ),
                Text(
                  t.protectionExceptions,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
        ),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(t.automaticUpdates),
                  subtitle: Text(t.updateDescription),
                  value: store.automaticUpdates,
                  onChanged: _busy
                      ? null
                      : (v) => _run(() async {
                          await store.setAutomaticUpdates(v);
                        }),
                ),
                if (_updateMessage != null) Text(_updateMessage!),
                Text('RouteLoom $currentReleaseVersion'),
                if (Platform.isIOS || Platform.isMacOS) Text(t.appleUpdates),
                TextButton(
                  onPressed: () => showLicensePage(
                    context: context,
                    applicationName: "RouteLoom",
                    applicationVersion: currentReleaseVersion,
                  ),
                  child: Text(t.licenses),
                ),
                TextButton(
                  onPressed: _checking ? null : _check,
                  child: Text(_checking ? t.checkingUpdates : t.checkUpdates),
                ),
                if (_download != null) ...[
                  Text(t.updateDownloaded),
                  SelectableText(_download!.path),
                  FilledButton(
                    onPressed: _busy
                        ? null
                        : () => _run(
                            () => _updates.install(
                              _update!,
                              _download!,
                              widget.disconnect,
                            ),
                          ),
                    child: Text(t.installUpdate),
                  ),
                ],
                if (_update != null)
                  TextButton(
                    onPressed: () =>
                        UpdateService.releasePage(_update!).catchError((_) {
                          _error(t.operationFailed);
                        }),
                    child: Text(t.openRelease),
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
