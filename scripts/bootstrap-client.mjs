// Generate official Flutter runners only when missing. Never overwrite native edits.
import {spawnSync} from 'node:child_process';
import {access, cp, mkdir} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const temporary=path.join(root,'.local','client-runners');
await mkdir(temporary,{recursive:true});
const result=spawnSync(process.platform==='win32'?'flutter.bat':'flutter',[
 'create','--empty','--no-pub','--project-name','routeloom',
 '--org','org.routeloom','--platforms=windows,macos,linux,android,ios',temporary,
],{stdio:'inherit',shell:process.platform==='win32'});
if(result.error) throw result.error;
if(result.status!==0) process.exit(result.status??1);
for(const entry of ['windows','macos','linux','android','ios','.metadata']) {
 const dest=path.join(root,'apps','client',entry);
 let exists=true;
 try {await access(dest);} catch {exists=false;}
 if(!exists) await cp(path.join(temporary,entry),dest,{recursive:true});
}
console.log('Client runners ready. Existing native integrations were preserved.');
