import 'dart:async';
import 'services/profile_store.dart';
import 'widgets/features.dart';
import 'widgets/traffic.dart';
import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'l10n/generated/app_localizations.dart';
import 'platforms/tunnel_adapter.dart';

void main() {
  LicenseRegistry.addLicense(() async* {
    yield LicenseEntryWithLineBreaks([
      'RouteLoom native dependencies',
    ], await rootBundle.loadString('assets/RUST-LICENSES.txt'));
    yield LicenseEntryWithLineBreaks([
      'China and LAN routing preset — V2Fly domain-list-community',
    ], await rootBundle.loadString('assets/DOMAIN-LIST-LICENSE.txt'));
  });
  runApp(const RouteLoomApp());
}

class RouteLoomApp extends StatefulWidget {
  const RouteLoomApp({super.key, this.store});
  final ProfileStore? store;
  @override
  State<RouteLoomApp> createState() => _RouteLoomAppState();
}

class _RouteLoomAppState extends State<RouteLoomApp> {
  Locale? _locale;
  late final ProfileStore _store = widget.store ?? ProfileStore();
  @override
  void initState() {
    super.initState();
    _restore();
  }

  Future<void> _restore() async {
    try {
      await _store.load();
      if (mounted)
        setState(
          () => _locale = _store.language == 'system'
              ? null
              : Locale(_store.language),
        );
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
      debugShowCheckedModeBanner: false,
      locale: _locale,
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff526e52)),
        scaffoldBackgroundColor: const Color(0xfff5f3eb),
        appBarTheme: const AppBarTheme(backgroundColor: Color(0xfff5f3eb)),
      ),
      home: RouteLoomHome(
        store: _store,
        language: _locale?.languageCode == 'zh'
            ? 'zh'
            : _locale?.languageCode ?? 'system',
        onLanguageChanged: (value) => setState(
          () => _locale = switch (value) {
            'en' => const Locale('en'),
            'zh' => const Locale('zh', 'CN'),
            _ => null,
          },
        ),
      ),
    );
  }
}

class RouteLoomHome extends StatefulWidget {
  const RouteLoomHome({
    super.key,
    required this.language,
    required this.store,
    required this.onLanguageChanged,
  });
  final ProfileStore store;
  final String language;
  final ValueChanged<String> onLanguageChanged;
  @override
  State<RouteLoomHome> createState() => _RouteLoomHomeState();
}

class _RouteLoomHomeState extends State<RouteLoomHome>
    with WidgetsBindingObserver {
  int _selected = 0;
  late final TunnelAdapter _adapter;
  StreamSubscription<TunnelState>? _subscription;
  TunnelState _state = TunnelState.disconnected;
  TunnelProfile? _sessionProfile;
  TunnelProfile? get _profile =>
      widget.store.active?.profile ?? _sessionProfile;
  bool get _monitoringSupported => [
    TargetPlatform.windows,
    TargetPlatform.android,
    TargetPlatform.linux,
    TargetPlatform.macOS,
    TargetPlatform.iOS,
  ].contains(defaultTargetPlatform);
  bool _changingMode = false;
  bool get _active => [
    TunnelState.connected,
    TunnelState.connecting,
    TunnelState.reconnecting,
    TunnelState.blocked,
    TunnelState.disconnecting,
  ].contains(_state);
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _adapter = createTunnelAdapter();
    _subscription = _adapter.states.listen((value) {
      if (mounted) setState(() => _state = value);
    });
    unawaited(_adapter.refresh());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) unawaited(_adapter.refresh());
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _subscription?.cancel();
    _adapter.dispose();
    super.dispose();
  }

  Future<void> _import() async {
    final t = AppLocalizations.of(context);
    try {
      final clipboard = await Clipboard.getData(Clipboard.kTextPlain);
      final profile = TunnelProfile.parse(clipboard?.text ?? '');
      if (widget.store.available) {
        await widget.store.import(profile);
      } else {
        _sessionProfile = profile;
        if (mounted)
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(SnackBar(content: Text(t.storageUnavailable)));
      }
      if (mounted) setState(() {});
    } catch (_) {
      if (mounted)
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(t.invalidProfile)));
    }
  }

  Future<void> _changeMode(String mode) async {
    final profile = _profile;
    if (_active || _changingMode || profile == null) return;
    setState(() => _changingMode = true);
    try {
      final updated = profile.withOptions({'mode': mode});
      if (widget.store.active != null)
        await widget.store.updateActive(updated);
      else
        _sessionProfile = updated;
      if (mounted) setState(() {});
    } catch (_) {
      if (mounted)
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context).operationFailed)),
        );
    } finally {
      if (mounted) setState(() => _changingMode = false);
    }
  }

  Future<void> _toggle() async {
    if (_changingMode) return;
    try {
      if (_state == TunnelState.connected ||
          _state == TunnelState.connecting ||
          _state == TunnelState.reconnecting) {
        await _adapter.disconnect();
      } else {
        await _adapter.connect(_profile!);
      }
    } catch (_) {
      if (mounted) setState(() => _state = TunnelState.failed);
    }
  }

  Widget _card(String title, String description, {Widget? extra}) {
    return Card(
      elevation: 0,
      color: Colors.white,
      margin: const EdgeInsets.only(bottom: 18),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 12),
            Text(description),
            if (extra != null) ...[const SizedBox(height: 20), extra],
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    final details = [
      t.windowsDetail,
      t.macosDetail,
      t.linuxDetail,
      t.androidDetail,
      t.iosDetail,
    ];
    final pages = <Widget>[
      Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _card(
            switch (_state) {
              TunnelState.connected => t.connected,
              TunnelState.connecting => t.connecting,
              TunnelState.reconnecting => t.reconnecting,
              TunnelState.blocked => t.trafficBlocked,
              TunnelState.disconnecting => t.disconnecting,
              TunnelState.failed => t.connectionFailed,
              _ => t.disconnected,
            },
            t.connectionDescription,
            extra: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _profile == null ? t.connectionUnavailable : t.profileReady,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 16),
                if (_monitoringSupported && _profile != null) ...[
                  ModePicker(
                    mode: _profile!.options['mode'] as String,
                    enabled: !_active && !_changingMode,
                    onChanged: _changeMode,
                  ),
                  const SizedBox(height: 16),
                ],
                FilledButton.icon(
                  key: const Key('connect-button'),
                  onPressed:
                      (_profile == null &&
                              _state != TunnelState.connected &&
                              _state != TunnelState.connecting &&
                              _state != TunnelState.reconnecting) ||
                          _state == TunnelState.disconnecting ||
                          _changingMode
                      ? null
                      : _toggle,
                  icon: const Icon(Icons.power_settings_new),
                  label: Text(
                    _state == TunnelState.connected ||
                            _state == TunnelState.connecting ||
                            _state == TunnelState.reconnecting
                        ? t.disconnect
                        : t.connect,
                  ),
                ),
                const SizedBox(height: 8),
                TextButton.icon(
                  onPressed: _active || _changingMode ? null : _import,
                  icon: const Icon(Icons.content_paste),
                  label: Text(t.importProfile),
                ),
                Text(
                  t.previewNotice,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
          FeaturePanel(
            store: widget.store,
            active: _active || _changingMode,
            onChanged: () => setState(() {}),
            state: _state,
            disconnect: _adapter.disconnect,
          ),
          if (!_monitoringSupported) _card(t.launchScope, t.launchDescription),
        ],
      ),
      if (_monitoringSupported)
        TrafficPanel(
          adapter: _adapter,
          active: _active,
          connected: _state == TunnelState.connected,
        )
      else
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              t.currentPlatform + ': ' + platformName(defaultTargetPlatform),
            ),
            const SizedBox(height: 18),
            for (var i = 0; i < clientPlatforms.length; i++)
              _card(
                platformName(clientPlatforms[i]),
                details[i],
                extra: Text(t.adapterPending),
              ),
            Text(t.appleLimit),
          ],
        ),
      Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _card(
            t.language,
            t.resetLanguage,
            extra: DropdownButton<String>(
              key: const Key('language-selector'),
              value: widget.language,
              isExpanded: true,
              items: [
                DropdownMenuItem(
                  value: 'system',
                  child: Text(t.systemLanguage),
                ),
                DropdownMenuItem(value: 'en', child: Text(t.english)),
                DropdownMenuItem(value: 'zh', child: Text(t.chinese)),
              ],
              onChanged: (value) {
                if (value != null) {
                  widget.onLanguageChanged(value);
                  widget.store.language = value;
                  unawaited(
                    widget.store.save().catchError((_) {
                      if (mounted)
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text(t.storageUnavailable)),
                        );
                    }),
                  );
                }
              },
            ),
          ),
          _card(t.privacy, t.privacyDescription),
        ],
      ),
    ];
    return Scaffold(
      appBar: AppBar(
        title: const Text('RouteLoom'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Center(child: Text(t.developmentLabel)),
          ),
        ],
      ),
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 880),
            child: ListView(
              padding: const EdgeInsets.all(24),
              children: [
                Text(
                  t.tagline,
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
                const SizedBox(height: 28),
                pages[_selected],
              ],
            ),
          ),
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selected,
        onDestinationSelected: (index) => setState(() => _selected = index),
        destinations: [
          NavigationDestination(
            icon: const Icon(Icons.route),
            label: t.overview,
          ),
          NavigationDestination(
            icon: Icon(
              _monitoringSupported
                  ? Icons.monitor_heart_outlined
                  : Icons.devices,
            ),
            label: _monitoringSupported ? t.traffic : t.platforms,
          ),
          NavigationDestination(
            icon: const Icon(Icons.settings_outlined),
            label: t.settings,
          ),
        ],
      ),
    );
  }
}
