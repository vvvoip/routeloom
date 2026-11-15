import base64
import importlib.util
import json
from pathlib import Path
import plistlib
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("apple_store_export", Path(__file__).with_name("apple-store-export.py"))
store = importlib.util.module_from_spec(spec)
spec.loader.exec_module(store)


class AppleStoreArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "packaging").mkdir()
        (self.root / "packaging/release.json").write_text(json.dumps({"version": "0.4.0", "sequence": 4}))
        (self.root / "apps/client").mkdir(parents=True)
        (self.root / "apps/client/pubspec.yaml").write_text("version: 0.4.0+4\n")
        self.config = self.root / "Flutter-Generated.xcconfig"
        self.policy = "https://routeloom.invalid-policy-domain.com/privacy"
        self.write_defines(self.policy)
        self.architectures = "arm64 x86_64"
        self.missing_symbol_arch = None
        self.signature_team = "ABCDE12345"
        self.extension_team = self.signature_team
        self.adhoc = False

    def write_defines(self, policy, store_enabled=True):
        values = ["ROUTELOOM_APPLE_STORE=" + str(store_enabled).lower(), "ROUTELOOM_PRIVACY_URL=" + policy]
        self.config.write_text("DART_DEFINES=" + ",".join(base64.b64encode(value.encode()).decode() for value in values) + "\n")

    def write_plist(self, file, value):
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(plistlib.dumps(value))

    def fixture(self, platform="macos"):
        self.archive = self.root / "RouteLoom.xcarchive"
        app_name = "routeloom.app" if platform == "macos" else "Runner.app"
        self.app = self.archive / "Products/Applications" / app_name
        self.content = self.app / "Contents" if platform == "macos" else self.app
        self.extension = self.content / "PlugIns/PacketTunnel.appex"
        self.extension_content = self.extension / "Contents" if platform == "macos" else self.extension
        self.app_info = {"CFBundleIdentifier": "com.owner.routeloom", "CFBundleExecutable": "routeloom",
                         "CFBundleShortVersionString": "0.4.0", "CFBundleVersion": "4",
                         "CFBundleSupportedPlatforms": ["MacOSX" if platform == "macos" else "iPhoneOS"],
                         "RouteLoomKeychainGroup": "ABCDE12345.com.owner.routeloom.vpn"}
        self.extension_info = dict(self.app_info, CFBundleIdentifier="com.owner.routeloom.PacketTunnel",
                                   CFBundleExecutable="PacketTunnel",
                                   NSExtension={"NSExtensionPointIdentifier": "com.apple.networkextension.packet-tunnel",
                                                "NSExtensionPrincipalClass": "PacketTunnel.PacketTunnelProvider"})
        self.write_plist(self.archive / "Info.plist", {"ApplicationProperties": {"ApplicationPath": "Applications/" + app_name}})
        self.write_plist(self.content / "Info.plist", self.app_info)
        self.write_plist(self.extension_content / "Info.plist", self.extension_info)
        for directory, name in ((self.content, "routeloom"), (self.extension_content, "PacketTunnel")):
            executable = directory / "MacOS" / name if platform == "macos" else directory / name
            executable.parent.mkdir(parents=True, exist_ok=True)
            executable.write_bytes(b"fixture; native tools are mocked")
        self.architectures = "arm64 x86_64" if platform == "macos" else "arm64"
        return self.archive

    def run_tool(self, arguments):
        if arguments[:3] == ["xcrun", "lipo", "-archs"]:
            return self.architectures, ""
        if arguments[:3] == ["xcrun", "nm", "-arch"]:
            names = ["_routeloom_snapshot", "_routeloom_command"]
            if arguments[3] == self.missing_symbol_arch:
                names.remove("_routeloom_command")
            return "\n".join("0000000100000000 T " + name for name in names), ""
        if arguments[:2] == ["codesign", "--verify"]:
            return "", ""
        if arguments[:3] == ["codesign", "--display", "--verbose=4"]:
            if self.adhoc:
                return "", "Signature=adhoc\n"
            team = self.extension_team if str(arguments[-1]).endswith(".appex") else self.signature_team
            return "", "TeamIdentifier=" + team + "\nAuthority=Apple Development: Test\n"
        if arguments[:3] == ["codesign", "--display", "--entitlements"]:
            return plistlib.dumps({"com.apple.developer.networking.networkextension": ["packet-tunnel-provider"],
                                   "keychain-access-groups": ["ABCDE12345.com.owner.routeloom.vpn"]}).decode(), ""
        self.fail("Unexpected tool: " + str(arguments))

    def verify(self, platform="macos", unsigned=True, policy=None):
        return store.verify_archive(self.root, platform, self.archive, self.config,
                                    self.policy if policy is None else policy, unsigned, run=self.run_tool)

    def test_unsigned_macos_and_ios_archives_are_never_reported_installable(self):
        for platform in ("macos", "ios"):
            with self.subTest(platform=platform):
                self.fixture(platform)
                report = self.verify(platform)
                self.assertFalse(report["customerInstallable"])
                self.assertFalse(report["uploaded"])
                self.assertEqual(report["signing"], "unsigned-ci")
                self.assertTrue(report["monitoringSymbolsPresent"])
                # Remove only the fixture through TemporaryDirectory; use another root for iOS.
                if platform == "macos":
                    self.archive.rename(self.root / "MacFixture.xcarchive")

    def test_rejects_stale_embedded_extension(self):
        self.fixture()
        self.extension_info["CFBundleVersion"] = "1"
        self.write_plist(self.extension_content / "Info.plist", self.extension_info)
        with self.assertRaisesRegex(store.ArchiveError, "Extension version"):
            self.verify()

    def test_rejects_wrong_extension_identity_or_keychain_group(self):
        self.fixture()
        for key, value, message in (("CFBundleIdentifier", "com.other.PacketTunnel", "identity"),
                                    ("RouteLoomKeychainGroup", "other.group", "Keychain")):
            original = self.extension_info[key]
            self.extension_info[key] = value
            self.write_plist(self.extension_content / "Info.plist", self.extension_info)
            with self.assertRaisesRegex(store.ArchiveError, message):
                self.verify()
            self.extension_info[key] = original

    def test_rejects_missing_monitor_function_in_either_universal_slice(self):
        self.fixture()
        for arch in ("arm64", "x86_64"):
            self.missing_symbol_arch = arch
            with self.assertRaisesRegex(store.ArchiveError, "entry points for " + arch):
                self.verify()

    def test_rejects_non_store_build_or_wrong_privacy_define(self):
        self.fixture()
        self.write_defines(self.policy, store_enabled=False)
        with self.assertRaisesRegex(store.ArchiveError, "Apple Store build"):
            self.verify()
        self.write_defines("https://another.real-domain.org/privacy")
        with self.assertRaisesRegex(store.ArchiveError, "privacy URL"):
            self.verify()

    def test_unsigned_ci_may_omit_policy_but_signed_archives_require_it(self):
        self.fixture()
        self.write_defines("")
        self.assertFalse(self.verify(policy="")["privacyPolicyConfigured"])
        with self.assertRaisesRegex(store.ArchiveError, "ROUTELOOM_PRIVACY_URL"):
            self.verify(unsigned=False, policy="")

    def test_rejects_placeholder_or_non_public_privacy_urls(self):
        for value in ("", "http://publisher.org/privacy", "https://example.com/privacy",
                      "https://localhost/privacy", "https://127.0.0.1/privacy",
                      "https://server.local/privacy", "https://server.internal/privacy",
                      "https://server.home.arpa/privacy",
                      "https://10.1.2.3/privacy", "https://user:password@publisher.org/privacy",
                      "https://policy.invalid/privacy", "https://www.example.com/privacy",
                      "https://publisher.org:badport/privacy"):
            with self.subTest(value=value):
                self.assertFalse(store.privacy_url_valid(value))
        self.assertTrue(store.privacy_url_valid("https://publisher.org/privacy"))

    def test_rejects_simulator_and_missing_universal_architecture(self):
        self.fixture()
        self.architectures = "x86_64"
        with self.assertRaisesRegex(store.ArchiveError, "architecture"):
            self.verify()
        self.app_info["CFBundleSupportedPlatforms"] = ["iPhoneSimulator"]
        self.write_plist(self.content / "Info.plist", self.app_info)
        with self.assertRaisesRegex(store.ArchiveError, "platform"):
            self.verify()

    def test_rejects_escaping_archive_application_path(self):
        self.fixture()
        self.write_plist(self.archive / "Info.plist", {"ApplicationProperties": {"ApplicationPath": "../../outside.app"}})
        with self.assertRaisesRegex(store.ArchiveError, "escapes"):
            self.verify()

    def test_signed_archive_checks_both_teams_and_rejects_adhoc_signing(self):
        self.fixture()
        report = self.verify(unsigned=False)
        self.assertEqual(report["teamIdentifier"], self.signature_team)
        self.assertFalse(report["customerInstallable"])
        self.extension_team = "OTHER12345"
        with self.assertRaisesRegex(store.ArchiveError, "teams differ"):
            self.verify(unsigned=False)
        self.adhoc = True
        with self.assertRaisesRegex(store.ArchiveError, "ad-hoc"):
            self.verify(unsigned=False)


if __name__ == "__main__":
    unittest.main()
