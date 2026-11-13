#!/usr/bin/env python3
"""Set local Apple bundle identities; never handles certificates or account passwords."""
import pathlib, re, sys
if len(sys.argv) != 3 or not re.fullmatch(r'[A-Z0-9]{10}', sys.argv[1]) or not re.fullmatch(r'[A-Za-z][A-Za-z0-9-]*(\.[A-Za-z][A-Za-z0-9-]*){2,}', sys.argv[2]):
    raise SystemExit('Usage: python3 scripts/prepare-apple.py TEAMID1234 com.yourname.routeloom')
root = pathlib.Path(__file__).resolve().parent.parent
team, bundle = sys.argv[1:]
for platform in ('ios', 'macos'):
    project = root / 'apps/client' / platform / 'Runner.xcodeproj/project.pbxproj'
    text = project.read_text()
    text = re.sub(r'DEVELOPMENT_TEAM = [^;]*;\s*', '', text)
    text = text.replace('buildSettings = {', 'buildSettings = { DEVELOPMENT_TEAM = '+team+';')
    text = text.replace('org.routeloom.routeloom', bundle)
    project.write_text(text)
config = root / 'apps/client/macos/Runner/Configs/AppInfo.xcconfig'
config.write_text(config.read_text().replace('org.routeloom.routeloom', bundle))
print('Apple team and bundle identifiers configured locally. Open Runner and check both Runner and PacketTunnel signing.')

for platform in ('ios', 'macos'):
    for file in (root / 'apps/client' / platform).rglob('*'):
        if file.suffix == '.entitlements' or file.name == 'Info.plist':
            file.write_text(file.read_text().replace('org.routeloom.routeloom', bundle))
