import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import '../l10n/generated/app_localizations.dart';
import '../services/distribution.dart';

class PublisherPrivacyLink extends StatelessWidget {
  const PublisherPrivacyLink({super.key});

  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    final uri = publisherPrivacyPolicy;
    if (uri == null) return Text(t.privacyPolicyUnavailable);
    return TextButton(
      key: const Key('publisher-privacy-policy'),
      onPressed: () async {
        try {
          if (!await launchUrl(uri, mode: LaunchMode.externalApplication)) {
            throw StateError('Cannot open privacy policy');
          }
        } catch (_) {
          if (context.mounted) {
            ScaffoldMessenger.of(
              context,
            ).showSnackBar(SnackBar(content: Text(t.privacyPolicyOpenFailed)));
          }
        }
      },
      child: Text(t.publisherPrivacyPolicy),
    );
  }
}

Future<bool> showVpnDataDisclosure(BuildContext context) async {
  final t = AppLocalizations.of(context);
  return await showDialog<bool>(
        context: context,
        barrierDismissible: false,
        builder: (context) => AlertDialog(
          title: Text(t.vpnDataTitle),
          content: SizedBox(
            width: 520,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(t.vpnDataRouting),
                  const SizedBox(height: 12),
                  Text(t.vpnDataStorage),
                  const SizedBox(height: 12),
                  Text(t.vpnDataServices),
                  const SizedBox(height: 12),
                  const PublisherPrivacyLink(),
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
              key: const Key('accept-vpn-disclosure'),
              onPressed: publisherPrivacyPolicy == null
                  ? null
                  : () => Navigator.pop(context, true),
              child: Text(t.agreeAndConnect),
            ),
          ],
        ),
      ) ??
      false;
}
