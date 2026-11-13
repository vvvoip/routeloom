import fs from 'node:fs/promises';
import path from 'node:path';
const source='.local/dependency-licenses';
let text='RouteLoom Rust dependency licenses\nGenerated from Cargo.lock. Flutter and plugin licenses are listed separately by Flutter.\n';
async function visit(dir){for(const entry of (await fs.readdir(dir,{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name))){const file=path.join(dir,entry.name);if(entry.isDirectory())await visit(file);else if(entry.name!=='inventory.json')text+='\n\n===== '+path.relative(source,file)+' =====\n'+await fs.readFile(file,'utf8');}}
await visit(source);
await fs.mkdir('apps/client/assets',{recursive:true});
await fs.writeFile('apps/client/assets/RUST-LICENSES.txt',text);
console.log('Bundled native dependency licenses');
