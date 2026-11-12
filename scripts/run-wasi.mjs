// Optional policy tests on hosts without a native linker. Not VPN platform validation.
import {WASI} from 'node:wasi';
import {readFile} from 'node:fs/promises';
const args=process.argv.slice(2);
const wasi=new WASI({version:'preview1',args,env:{},preopens:{'/workspace':process.cwd()}});
const module=await WebAssembly.compile(await readFile(args[0]));
const instance=await WebAssembly.instantiate(module,wasi.getImportObject());
wasi.start(instance);
