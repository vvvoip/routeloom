import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';
import {isIP} from 'node:net';
import {spawnSync} from 'node:child_process';
const [outArg, endpoint='127.0.0.1:9443', name='localhost'] = process.argv.slice(2);
if (!outArg || !/^[A-Za-z0-9.-]{1,253}$/.test(name)) {
  throw Error('Usage: node scripts/create-dev-profile.mjs OUTPUT_DIRECTORY [IP:PORT] [TLS_HOSTNAME]');
}
const split=endpoint.lastIndexOf(':');
const host=endpoint.slice(0,split).replace(/^\[|\]$/g,'');
const port=Number(endpoint.slice(split+1));
if(split<1 || !isIP(host) || !Number.isInteger(port) || port<1 || port>65535)throw Error('Endpoint must be numeric IP:PORT (IPv6 in brackets)');
const out=path.resolve(outArg);
await fs.mkdir(out,{recursive:true,mode:0o700});
if ((await fs.readdir(out)).length) throw Error('Output directory must be empty');
const generated=spawnSync('openssl',['req','-x509','-newkey','ec','-pkeyopt','ec_paramgen_curve:P-256','-noenc',
  '-days','30','-subj','/CN='+name,'-addext','subjectAltName=DNS:'+name,
  '-addext','basicConstraints=critical,CA:FALSE','-keyout',path.join(out,'server-key.pem'),'-out',path.join(out,'server-cert.pem')],{stdio:'pipe'});
if(generated.status!==0)throw Error('OpenSSL certificate generation failed; install OpenSSL 3 first');
await fs.chmod(path.join(out,'server-key.pem'),0o600);
const token=crypto.randomBytes(32).toString('hex');
const profile={endpoint,server_name:name,ca_pem:await fs.readFile(path.join(out,'server-cert.pem'),'utf8'),
  device:'device-1',token,ipv4:'10.77.0.2',ipv6:'fd77::2',routes:['0.0.0.0/0','::/0'],dns:['1.1.1.1','2606:4700:4700::1111']};
const server={listen:(isIP(host)===6?'[::]:':'0.0.0.0:')+port,certificate:'server-cert.pem',private_key:'server-key.pem',
  devices:[{id:profile.device,token_sha256:crypto.createHash('sha256').update(token).digest('hex'),ipv4:profile.ipv4,ipv6:profile.ipv6}],allow_private:[]};
for(const [name,value] of [['device.profile.json',profile],['server.json',server]])
  await fs.writeFile(path.join(out,name),JSON.stringify(value,null,2)+'\n',{mode:0o600,flag:'wx'});
console.log('Created private development files in '+out+'. Transfer the profile securely; never commit it.');
