import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'l10n/generated/app_localizations.dart';
import 'platforms/tunnel_adapter.dart';

void main() => runApp(const RouteLoomApp());

class RouteLoomApp extends StatefulWidget {
  const RouteLoomApp({super.key});
  @override
  State<RouteLoomApp> createState() => _RouteLoomAppState();
}
class _RouteLoomAppState extends State<RouteLoomApp> {
  Locale? _locale;
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
        language: _locale?.languageCode == 'zh' ? 'zh' : _locale?.languageCode ?? 'system',
        onLanguageChanged: (value) => setState(() => _locale = switch (value) {
          'en' => const Locale('en'), 'zh' => const Locale('zh', 'CN'), _ => null,
        }),
      ),
    );
  }
}
class RouteLoomHome extends StatefulWidget {
  const RouteLoomHome({super.key, required this.language, required this.onLanguageChanged});
  final String language;
  final ValueChanged<String> onLanguageChanged;
  @override
  State<RouteLoomHome> createState() => _RouteLoomHomeState();
}
class _RouteLoomHomeState extends State<RouteLoomHome> {
  int _selected = 0;
  Widget _card(String title, String description, {Widget? extra}) {
    return Card(
      elevation: 0, color: Colors.white,
      margin: const EdgeInsets.only(bottom: 18),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(title, style: Theme.of(context).textTheme.titleLarge),
          const SizedBox(height: 12), Text(description),
          if (extra != null) ...[const SizedBox(height: 20), extra],
        ]),
      ),
    );
  }
  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    final details = [t.windowsDetail, t.macosDetail, t.linuxDetail, t.androidDetail, t.iosDetail];
    final pages = <Widget>[
      Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        _card(t.disconnected, t.connectionDescription, extra: Column(
          crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(t.connectionUnavailable, style: const TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 16),
            FilledButton.icon(onPressed: null, icon: const Icon(Icons.power_settings_new), label: Text(t.connect)),
            const SizedBox(height: 8),
            Text(t.previewNotice, style: Theme.of(context).textTheme.bodySmall),
          ],
        )),
        _card(t.routePolicy, t.routePolicyDescription),
        _card(t.launchScope, t.launchDescription),
      ]),
      Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Text(t.currentPlatform + ': ' + platformName(defaultTargetPlatform)),
        const SizedBox(height: 18),
        for (var i = 0; i < clientPlatforms.length; i++)
          _card(platformName(clientPlatforms[i]), details[i], extra: Text(t.adapterPending)),
        Text(t.appleLimit),
      ]),
      Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        _card(t.language, t.resetLanguage, extra: DropdownButton<String>(
          key: const Key('language-selector'), value: widget.language, isExpanded: true,
          items: [
            DropdownMenuItem(value: 'system', child: Text(t.systemLanguage)),
            DropdownMenuItem(value: 'en', child: Text(t.english)),
            DropdownMenuItem(value: 'zh', child: Text(t.chinese)),
          ],
          onChanged: (value) { if (value != null) widget.onLanguageChanged(value); },
        )),
        _card(t.privacy, t.privacyDescription),
      ]),
    ];
    return Scaffold(
      appBar: AppBar(title: const Text('RouteLoom'), actions: [
        Padding(padding: const EdgeInsets.only(right: 16), child: Center(child: Text(t.developmentLabel))),
      ]),
      body: SafeArea(child: Center(child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 880),
        child: ListView(padding: const EdgeInsets.all(24), children: [
          Text(t.tagline, style: Theme.of(context).textTheme.headlineMedium),
          const SizedBox(height: 28), pages[_selected],
        ]),
      ))),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selected,
        onDestinationSelected: (index) => setState(() => _selected = index),
        destinations: [
          NavigationDestination(icon: const Icon(Icons.route), label: t.overview),
          NavigationDestination(icon: const Icon(Icons.devices), label: t.platforms),
          NavigationDestination(icon: const Icon(Icons.settings_outlined), label: t.settings),
        ],
      ),
    );
  }
}
