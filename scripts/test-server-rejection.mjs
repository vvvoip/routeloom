import fs from 'node:fs/promises';
import tls from 'node:tls';
import assert from 'node:assert/strict';
const [profilePath,mode]=process.argv.slice(2);
const p=JSON.parse(await fs.readFile(profilePath,'utf8'));
const host=p.endpoint.slice(0,p.endpoint.lastIndexOf(':'));
const port=Number(p.endpoint.slice(p.endpoint.lastIndexOf(':')+1));
const frame=(kind,data)=>{const h=Buffer.alloc(5);h.writeUInt32BE(data.length+1);h[4]=kind;return Buffer.concat([h,data]);};
await new Promise((resolve,reject)=>{
 let buffer=Buffer.alloc(0), welcomed=false, ended=false, secured=false;
 const socket=tls.connect({host,port,servername:p.server_name,ca:p.ca_pem,minVersion:'TLSv1.3',maxVersion:'TLSv1.3',ALPNProtocols:['routeloom/1']});
 socket.setTimeout(10000,()=>{socket.destroy();reject(Error('Server did not reject invalid client'));});
 socket.on('secureConnect',()=>{
   secured=true;
   const token=mode==='token'?'0'.repeat(64):p.token;
   socket.write(frame(1,Buffer.from(JSON.stringify({version:1,device:p.device,token}))));
 });
 socket.on('data',data=>{
   buffer=Buffer.concat([buffer,data]);
   if(buffer.length<5)return;
   const n=buffer.readUInt32BE();
   if(n>4096){socket.destroy();reject(Error('Unbounded server response'));return;}
   if(buffer.length<n+4)return;
   if(mode!=='spoof'){socket.destroy();reject(Error('Invalid session was accepted'));return;}
   assert.equal(buffer[4],2);
   if(welcomed){socket.destroy();reject(Error('Spoofed packet was accepted'));return;}
   welcomed=true;buffer=buffer.subarray(n+4);
   const ip=Buffer.alloc(20);ip[0]=0x45;ip.writeUInt16BE(20,2);ip.set([10,77,0,3],12);ip.set([10,77,0,1],16);
   socket.write(frame(3,ip));
 });
 socket.on('end',()=>{ended=true;socket.destroy();});
 socket.on('close',()=> {
   if(mode==='spoof'&&!welcomed)reject(Error('Did not reach authenticated spoof test'));
   else if(ended)resolve();
   else reject(Error('Unexpected transport close'));
 });
 socket.on('error',()=> { if(secured && (mode!=='spoof'||welcomed)) {ended=true;} else reject(Error('TLS test connection failed')); });
});
console.log('Server rejected '+mode+' as expected.');
