// Enabled only for Apple archives intended for TestFlight / App Store.
const appleStoreDistribution = bool.fromEnvironment('ROUTELOOM_APPLE_STORE');
const publisherPrivacyPolicyUrl = String.fromEnvironment(
  'ROUTELOOM_PRIVACY_URL',
);

Uri? get publisherPrivacyPolicy {
  final uri = Uri.tryParse(publisherPrivacyPolicyUrl);
  if (uri == null ||
      uri.scheme != 'https' ||
      uri.host.isEmpty ||
      uri.userInfo.isNotEmpty)
    return null;
  return uri;
}

void requireExternalUpdates() {
  if (appleStoreDistribution) {
    throw StateError('This build receives updates through Apple.');
  }
}
