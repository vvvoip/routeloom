import fs from 'node:fs/promises';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const destination=process.argv[2];
if(!destination) throw Error('Usage: node scripts/collect-licenses.mjs OUTPUT');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--format-version','1'],{maxBuffer:32*1024*1024}));
await fs.mkdir(destination,{recursive:true});
const inventory=[];
for(const pkg of metadata.packages.filter(p=>p.source)) {
 const source=path.dirname(pkg.manifest_path), dir=path.join(destination,pkg.name+'-'+pkg.version);
 await fs.mkdir(dir,{recursive:true});
 const files=[];
 for(const item of await fs.readdir(source,{withFileTypes:true})) {
  if(/^(licen[sc]e|copying|notice|copyright)/i.test(item.name)) {
   await fs.cp(path.join(source,item.name),path.join(dir,item.name),{recursive:true}); files.push(item.name);
  }
 }
 if(pkg.license_file && !files.includes(pkg.license_file)) {
  const license=path.resolve(source,pkg.license_file);
  if(!license.startsWith(source+path.sep)) throw Error('License escapes package');
  await fs.cp(license,path.join(dir,path.basename(license)));
  files.push(pkg.license_file);
 }
 if(pkg.name==='wintun-bindings') {
  await fs.cp(path.join(source,'wintun','LICENSE.txt'),path.join(dir,'WINTUN-LICENSE.txt'));
  files.push('WINTUN-LICENSE.txt');
 }
 if(!files.length && pkg.name==='tun') {
  const sourceText=await fs.readFile(path.join(source,'src/lib.rs'),'utf8');
  const header=sourceText.split('\n\n')[0];
  if(!header.includes('PUBLIC LICENSE')) throw Error('TUN license header missing');
  await fs.writeFile(path.join(dir,'LICENSE.txt'),header.replace(/^\/\/ ?/gm,'')+'\n');
  files.push('LICENSE.txt (from src/lib.rs)');
 }
 if(!files.length && pkg.name==='asn1-rs-impl' && pkg.version==='0.2.0') {
  await fs.cp('packaging/licenses/asn1-rs-impl-0.2.0',dir,{recursive:true});
  files.push('LICENSE-MIT','SOURCE.txt');
 }
 if(!files.length && pkg.name==='etherparse' && pkg.version==='0.16.0') {
  await fs.cp('packaging/licenses/etherparse-0.16.0',dir,{recursive:true});files.push('LICENSE-MIT','SOURCE.txt');
 }
 if(!files.length && pkg.name==='jni-sys-macros' && pkg.version==='0.4.1') {await fs.cp('packaging/licenses/jni-sys-macros-0.4.1',dir,{recursive:true});files.push('LICENSE-MIT','SOURCE.txt');}
 if(!files.length && pkg.name==='r-efi' && pkg.version==='5.3.0') {const authors=await fs.readFile(path.join(source,'AUTHORS'),'utf8');if(!authors.includes('Permission is hereby granted'))throw Error('r-efi license missing');await fs.writeFile(path.join(dir,'AUTHORS-LICENSE.txt'),authors);files.push('AUTHORS-LICENSE.txt');}
 if(!files.length) throw Error('No license found for '+pkg.name);
 inventory.push({name:pkg.name,version:pkg.version,license:pkg.license,repository:pkg.repository,files});
}
await fs.writeFile(path.join(destination,'inventory.json'),JSON.stringify(inventory,null,2)+'\n');
console.log('Collected licenses for '+inventory.length+' Rust dependencies');
