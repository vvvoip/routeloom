// External RL/1 smoke test. Sends IP packets over TLS without changing local routes or DNS.
import fs from 'node:fs/promises';
import tls from 'node:tls';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';

const [profilePath] = process.argv.slice(2);
if (!profilePath) throw Error('Usage: node scripts/probe-server.mjs PRIVATE_PROFILE.json');
const p = JSON.parse(await fs.readFile(profilePath, 'utf8'));
const split = p.endpoint.lastIndexOf(':');
const host = p.endpoint.slice(0, split).replace(/^\[|\]$/g, '');
const port = Number(p.endpoint.slice(split + 1));
const socket = tls.connect({host, port, servername:p.server_name, ca:p.ca_pem,
  minVersion:'TLSv1.3', maxVersion:'TLSv1.3', ALPNProtocols:['routeloom/1']});
const frames = [], waiters = [];
let buffered = Buffer.alloc(0), failure;
const fail = error => {
  failure = error;
  for (const waiter of waiters.splice(0)) { clearTimeout(waiter.timer); waiter.reject(error); }
};
socket.on('error', () => fail(Error('TLS transport failed')));
socket.on('end', () => fail(Error('Server closed connection')));
socket.on('data', data => {
  buffered = Buffer.concat([buffered, data]);
  if (buffered.length > 65536) { socket.destroy(); fail(Error('Receive buffer limit')); return; }
  while (buffered.length >= 4) {
    const n = buffered.readUInt32BE();
    if (n < 1 || n > 4096) { socket.destroy(); fail(Error('Invalid frame')); return; }
    if (buffered.length < n+4) break;
    const frame = [buffered[4], buffered.subarray(5,n+4)];
    buffered = buffered.subarray(n+4);
    const waiter = waiters.shift();
    if (waiter) { clearTimeout(waiter.timer); waiter.resolve(frame); }
    else if (frames.length < 64) frames.push(frame);
    else { socket.destroy(); fail(Error('Frame queue limit')); return; }
  }
});
function readFrame() {
  if (frames.length) return Promise.resolve(frames.shift());
  if (failure) return Promise.reject(failure);
  return new Promise((resolve,reject) => {
    const waiter = {resolve,reject};
    waiter.timer = setTimeout(() => {
      const index=waiters.indexOf(waiter); if(index>=0) waiters.splice(index,1);
      reject(Error('Response timeout'));
    },10000);
    waiters.push(waiter);
  });
}
function sendFrame(kind,data=Buffer.alloc(0)) {
  const header=Buffer.alloc(5);header.writeUInt32BE(data.length+1);header[4]=kind;
  socket.write(Buffer.concat([header,data]));
}
function checksum(data) {
  let sum=0; for(let i=0;i<data.length;i+=2) sum+=(data[i]<<8)+(data[i+1]??0);
  while(sum>>>16) sum=(sum&65535)+(sum>>>16);
  return (~sum)&65535;
}
function ipv4(destination,protocol,payload) {
  const h=Buffer.alloc(20);h[0]=0x45;h.writeUInt16BE(20+payload.length,2);
  h[8]=64;h[9]=protocol;
  h.set(p.ipv4.split('.').map(Number),12);h.set(destination.split('.').map(Number),16);
  h.writeUInt16BE(checksum(h),10);
  return Buffer.concat([h,payload]);
}
async function packet(match) {
  const deadline=Date.now()+10000;
  while(Date.now()<deadline) {
    const [kind,data]=await readFrame();
    if(kind===5) continue;
    if(kind!==3 || data.length<20 || data.length>1280 || data[0]>>>4!==4) throw Error('Invalid packet response');
    if(data.subarray(16,20).join('.')!==p.ipv4) throw Error('Wrong destination lease');
    const offset=(data[0]&15)*4;
    if(data.readUInt16BE(2)!==data.length || offset<20 || offset>data.length) throw Error('Invalid packet length');
    if(match(data,offset))return data.subarray(offset);
  }
  throw Error('Packet response timeout');
}
try {
  await new Promise((resolve,reject) => {
    const timer=setTimeout(()=>reject(Error('Connect timeout')),15000);
    socket.once('secureConnect',()=>{clearTimeout(timer);resolve();});
    socket.once('error',()=>{clearTimeout(timer);reject(Error('TLS authentication failed'));});
  });
  assert.equal(socket.alpnProtocol,'routeloom/1');
  sendFrame(1,Buffer.from(JSON.stringify({version:1,device:p.device,token:p.token})));
  const [kind,welcome]=await readFrame();
  assert.equal(kind,2);
  const lease=JSON.parse(welcome);
  assert.equal(lease.ipv4,p.ipv4);assert.equal(lease.mtu,1280);assert.equal(lease.version,1);
  const dnsResults=[];
  for(const dns of p.dns.filter(ip=>!ip.includes(':'))) {
    const id=crypto.randomInt(65536),sourcePort=crypto.randomInt(40000,60000);
    const query=Buffer.from('000001000001000000000000076578616d706c6503636f6d0000010001','hex');
    query.writeUInt16BE(id);
    const udp=Buffer.alloc(8);udp.writeUInt16BE(sourcePort);udp.writeUInt16BE(53,2);udp.writeUInt16BE(query.length+8,4);
    sendFrame(3,ipv4(dns,17,Buffer.concat([udp,query])));
    const response=await packet((data,offset)=>data[9]===17&&data.subarray(12,16).join('.')===dns&&data.readUInt16BE(offset+2)===sourcePort);
    assert.equal(response.readUInt16BE(8),id);
    assert.equal(response[11]&15,0);
    assert.ok(response.readUInt16BE(14)>0);
    dnsResults.push({resolver:dns,answers:response.readUInt16BE(14)});
  }
  sendFrame(4);
  const [pong,payload]=await readFrame();assert.equal(pong,5);assert.equal(payload.length,0);
  console.log(JSON.stringify({tls:socket.getProtocol(),alpn:socket.alpnProtocol,authenticated:true,dns:dnsResults,keepalive:true}));
} finally { socket.destroy(); }
