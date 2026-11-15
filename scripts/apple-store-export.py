#!/usr/bin/env python3
"""Check Apple store archive inputs/results; never export credentials or upload."""
import argparse
import base64
import ipaddress
import json
from pathlib import Path
import plistlib
import re
import subprocess
from urllib.parse import urlsplit


class ArchiveError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise ArchiveError(message)


def privacy_url_valid(value):
    try:
        url = urlsplit(value)
        # Accessing port also rejects malformed/non-numeric port syntax.
        _ = url.port
        host = (url.hostname or "").lower().rstrip(".")
        if url.scheme != "https" or not host or url.username or url.password:
            return False
        if host == "localhost" or any(host == example or host.endswith("." + example)
                                      for example in ("example.com", "example.org", "example.net")):
            return False
        if host.endswith((".localhost", ".local", ".internal", ".home.arpa", ".test", ".invalid", ".example")):
            return False
        try:
            return ipaddress.ip_address(host).is_global
        except ValueError:
            return "." in host and not any(ch.isspace() for ch in value)
    except ValueError:
        return False


def release_version(root):
    release = json.loads((root / "packaging/release.json").read_text(encoding="utf-8"))
    match = re.search(r"^version:\s*(\d+\.\d+\.\d+)\+(\d+)\s*$",
                      (root / "apps/client/pubspec.yaml").read_text(encoding="utf-8"), re.M)
    require(match is not None, "Client version is missing")
    expected = (release["version"], str(release["sequence"]))
    require(match.groups() == expected, "Client and release versions differ")
    return expected


def preflight(root, privacy_url, unsigned):
    require(unsigned or privacy_url_valid(privacy_url),
            "Set ROUTELOOM_PRIVACY_URL to the approved public HTTPS privacy-policy URL before signing")
    require(not privacy_url or privacy_url_valid(privacy_url),
            "Privacy-policy URL must be public HTTPS, without credentials or a placeholder hostname")
    version, build = release_version(root)
    return {"version": version, "build": build, "signing": "unsigned-ci" if unsigned else "owner-signed",
            "privacyPolicyConfigured": bool(privacy_url)}


def within(base, relative):
    candidate = (base / relative).resolve()
    require(candidate.is_relative_to(base.resolve()), "Archive path escapes its bundle")
    return candidate


def read_plist(path):
    require(path.is_file(), "Missing archive plist: " + path.name)
    with path.open("rb") as handle:
        value = plistlib.load(handle)
    require(isinstance(value, dict), "Archive plist is not a dictionary")
    return value


def identity(info, label):
    identifier = info.get("CFBundleIdentifier", "")
    require(isinstance(identifier, str) and re.fullmatch(r"[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+", identifier),
            label + " bundle identifier is missing or unresolved")
    executable = info.get("CFBundleExecutable", "")
    require(isinstance(executable, str) and executable not in {"", ".", ".."}
            and "/" not in executable and "\\" not in executable and "$(" not in executable,
            label + " executable name is invalid")
    return identifier, executable


def run_tool(arguments):
    result = subprocess.run(arguments, text=True, capture_output=True, check=False)
    require(result.returncode == 0, "Archive tool failed: " + arguments[0])
    return result.stdout, result.stderr


def verify_signature(bundle, run):
    run(["codesign", "--verify", "--deep", "--strict", str(bundle)])
    output, errors = run(["codesign", "--display", "--verbose=4", str(bundle)])
    description = output + "\n" + errors
    team = re.search(r"^TeamIdentifier=([A-Z0-9]{10})$", description, re.M)
    require(team is not None and "Signature=adhoc" not in description,
            "Archive must use an Apple signing identity; ad-hoc signing is not sufficient")
    output, _ = run(["codesign", "--display", "--entitlements", ":-", str(bundle)])
    entitlements = plistlib.loads(output.encode())
    require("packet-tunnel-provider" in entitlements.get("com.apple.developer.networking.networkextension", []),
            "Signed app or extension lacks the packet-tunnel entitlement")
    return team.group(1), entitlements


def verify_archive(root, platform, archive, flutter_config, privacy_url, unsigned, run=run_tool):
    report = preflight(root, privacy_url, unsigned)
    archive = archive.resolve()
    archive_info = read_plist(archive / "Info.plist")
    application_path = archive_info.get("ApplicationProperties", {}).get("ApplicationPath")
    require(isinstance(application_path, str), "Archive is not an application archive")
    products = archive / "Products"
    app = within(products, application_path)
    require(app.parent == (products / "Applications").resolve() and app.suffix == ".app" and app.is_dir(),
            "Archive must contain a top-level application")
    require(len(list((products / "Applications").glob("*.app"))) == 1,
            "Archive contains multiple top-level applications")
    content = app / "Contents" if platform == "macos" else app
    app_info = read_plist(content / "Info.plist")
    app_id, app_executable = identity(app_info, "App")
    expected_platform = "MacOSX" if platform == "macos" else "iPhoneOS"
    require(expected_platform in app_info.get("CFBundleSupportedPlatforms", []),
            "Archive platform is wrong or a simulator build")
    extension = content / "PlugIns/PacketTunnel.appex"
    extension_content = extension / "Contents" if platform == "macos" else extension
    extension_info = read_plist(extension_content / "Info.plist")
    extension_id, extension_executable = identity(extension_info, "Extension")
    require(extension_id == app_id + ".PacketTunnel", "Embedded extension identity does not match the app")
    expected_version = (report["version"], report["build"])
    for label, info in (("App", app_info), ("Extension", extension_info)):
        require((info.get("CFBundleShortVersionString"), info.get("CFBundleVersion")) == expected_version,
                label + " version differs from the release")
    declaration = extension_info.get("NSExtension", {})
    require(declaration.get("NSExtensionPointIdentifier") == "com.apple.networkextension.packet-tunnel",
            "PacketTunnel extension point is missing")
    require(str(declaration.get("NSExtensionPrincipalClass", "")).endswith(".PacketTunnelProvider"),
            "PacketTunnel principal class is missing")
    group = app_info.get("RouteLoomKeychainGroup")
    require(isinstance(group, str) and bool(group) and "$(" not in group
            and extension_info.get("RouteLoomKeychainGroup") == group,
            "App and extension Keychain group metadata differs or is unresolved")

    config = flutter_config.read_text(encoding="utf-8")
    define_lines = re.findall(r"^DART_DEFINES=(.*)$", config, re.M)
    require(len(define_lines) == 1, "Generated Flutter configuration must have one DART_DEFINES value")
    try:
        defines = [base64.b64decode(item, validate=True).decode("utf-8")
                   for item in define_lines[0].strip().split(",") if item]
    except (ValueError, UnicodeDecodeError) as error:
        raise ArchiveError("Generated Dart defines are invalid") from error
    require(defines.count("ROUTELOOM_APPLE_STORE=true") == 1,
            "Archive configuration does not enable the Apple Store build")
    require(defines.count("ROUTELOOM_PRIVACY_URL=" + privacy_url) == 1,
            "Archive configuration does not match the supplied privacy URL")
    require(not any(item.startswith("ROUTELOOM_APPLE_STORE=") and item != "ROUTELOOM_APPLE_STORE=true"
                    for item in defines), "Conflicting Apple Store build flags")
    require(sum(item.startswith("ROUTELOOM_PRIVACY_URL=") for item in defines) == 1,
            "Conflicting privacy-policy build flags")

    binaries = [("App", within(content, "MacOS/" + app_executable if platform == "macos" else app_executable)),
                ("Extension", within(extension_content, "MacOS/" + extension_executable if platform == "macos" else extension_executable))]
    expected_arches = {"arm64", "x86_64"} if platform == "macos" else {"arm64"}
    for label, executable in binaries:
        require(executable.is_file(), label + " binary is missing")
        architectures, _ = run(["xcrun", "lipo", "-archs", str(executable)])
        require(set(architectures.strip().split()) == expected_arches, label + " architecture set is incorrect")
        if label == "Extension":
            for arch in sorted(expected_arches):
                symbols, _ = run(["xcrun", "nm", "-arch", arch, "-g", "-U", str(executable)])
                found = {line.split()[-1] for line in symbols.splitlines() if line.split()}
                require({"_routeloom_snapshot", "_routeloom_command"}.issubset(found),
                        "Embedded extension lacks monitoring entry points for " + arch)
    if not unsigned:
        app_team, app_entitlements = verify_signature(app, run)
        extension_team, extension_entitlements = verify_signature(extension, run)
        require(app_team == extension_team, "App and extension signing teams differ")
        for entitlements in (app_entitlements, extension_entitlements):
            require(group in entitlements.get("keychain-access-groups", []),
                    "Signed Keychain access group does not match metadata")
        report["teamIdentifier"] = app_team
    report.update({"platform": platform, "bundleIdentifier": app_id,
                   "extensionBundleIdentifier": extension_id, "architectures": sorted(expected_arches),
                   "monitoringSymbolsPresent": True, "appleStoreBuildConfigured": True,
                   "customerInstallable": False, "uploaded": False,
                   "validationScope": "archive structure and local signatures only; not App Store validation",
                   "remaining": ["Organizer distribution signing and validation", "App Store Connect privacy and export compliance",
                                 "device and VPN testing", "Apple review and customer distribution"]})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("preflight", "verify"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--privacy-url", default="")
    parser.add_argument("--unsigned", action="store_true")
    parser.add_argument("--platform", choices=("macos", "ios"))
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--flutter-config", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    try:
        if args.action == "preflight":
            report = preflight(args.root, args.privacy_url, args.unsigned)
        else:
            require(all((args.platform, args.archive, args.flutter_config)), "verify needs platform, archive and Flutter configuration")
            report = verify_archive(args.root, args.platform, args.archive, args.flutter_config,
                                    args.privacy_url, args.unsigned)
        text = json.dumps(report, indent=2) + "\n"
        if args.report:
            args.report.write_text(text, encoding="utf-8")
        print(text, end="")
    except (ArchiveError, OSError, plistlib.InvalidFileException, json.JSONDecodeError) as error:
        parser.exit(1, "Apple archive check failed: " + str(error) + "\n")


if __name__ == "__main__":
    main()
