import fs from 'node:fs/promises';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const platform=process.argv[2];
const root=process.cwd(), out=path.join(root,'.local','release-assets');
const config=JSON.parse(await fs.readFile('packaging/release.json','utf8'));
await fs.mkdir(out,{recursive:true});
const stage=path.join(root,'.local','package-'+platform);
await fs.mkdir(stage,{recursive:true});
const copy=async(from,to)=>{await fs.mkdir(path.dirname(to),{recursive:true});await fs.cp(from,to,{recursive:true});};
async function notices(dir) {
 for(const name of ['LICENSE','NOTICE','THIRD_PARTY_NOTICES.md']) await copy(name,path.join(dir,name));
 await copy('docs/install.md',path.join(dir,'READ-ME.md'));
 await copy('docs/install.zh-CN.md',path.join(dir,'使用说明.md'));
 await copy('.local/dependency-licenses',path.join(dir,'licenses'));
}
if(platform==='windows') {
 await copy('apps/client/build/windows/x64/runner/Release',stage);
 await copy('target/release/routeloom-vpn.exe',path.join(stage,'routeloom-vpn.exe'));
 const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--format-version','1'],{maxBuffer:32*1024*1024}));
 const wintun=metadata.packages.find(p=>p.name==='wintun-bindings');
 if(!wintun) throw Error('Pinned Wintun dependency is missing');
 await copy(path.join(path.dirname(wintun.manifest_path),'wintun/bin/amd64/wintun.dll'),path.join(stage,'wintun.dll'));
 await copy('packaging/windows/Start-RouteLoom.cmd',path.join(stage,'Start-RouteLoom.cmd'));
 await notices(stage);
 for(const name of ['routeloom.exe','routeloom-vpn.exe','wintun.dll','flutter_windows.dll','vcruntime140.dll','msvcp140.dll']) await fs.access(path.join(stage,name));
 execFileSync('powershell.exe',['-NoProfile','-NonInteractive','-Command', 'Compress-Archive -Path (Join-Path $env:PACKAGE_STAGE "*") -DestinationPath $env:PACKAGE_ARCHIVE -Force'],{stdio:'inherit',env:{...process.env,PACKAGE_STAGE:stage,PACKAGE_ARCHIVE:path.join(out,'routeloom-windows-x64.zip')}});
} else if(platform==='linux') {
 const bundle=path.join(stage,'usr/lib/routeloom');
 await copy('apps/client/build/linux/x64/release/bundle',bundle);
 await copy('target/release/routeloom-vpn',path.join(bundle,'routeloom-vpn'));
 await fs.chmod(path.join(bundle,'routeloom-vpn'),0o755);
 await notices(path.join(stage,'usr/share/doc/routeloom'));
 await copy('packaging/linux/routeloom.desktop',path.join(stage,'usr/share/applications/routeloom.desktop'));
 await copy('packaging/linux/org.routeloom.vpn.policy',path.join(stage,'usr/share/polkit-1/actions/org.routeloom.vpn.policy'));
 await fs.mkdir(path.join(stage,'DEBIAN'),{recursive:true});
 await fs.writeFile(path.join(stage,'DEBIAN/control'),'Package: routeloom\nVersion: '+config.debianVersion+'\nArchitecture: amd64\nMaintainer: RouteLoom <noreply@github.com>\nSection: net\nPriority: optional\nDepends: libc6 (>= 2.39), libgtk-3-0t64, libstdc++6, liblzma5, libglu1-mesa, libsecret-1-0, libjsoncpp25, nftables, iproute2, systemd-resolved, pkexec, policykit-1\nDescription: RouteLoom encrypted VPN preview\n Bilingual client for a self-hosted RouteLoom server.\n');
 execFileSync('dpkg-deb',['--root-owner-group','--build',stage,path.join(out,'routeloom-linux-amd64.deb')],{stdio:'inherit'});
 execFileSync('dpkg-deb',['--info',path.join(out,'routeloom-linux-amd64.deb')],{stdio:'inherit'});
} else if(platform==='android') {
 await copy('apps/client/build/app/outputs/flutter-apk/app-release.apk',path.join(out,'routeloom-android.apk'));
} else throw Error('Unknown platform '+platform);
console.log('Packaged '+platform);
