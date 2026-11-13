import {readFileSync,writeFileSync,statSync} from 'node:fs';
import {createHash,createPrivateKey,sign,verify,createPublicKey} from 'node:crypto';
import {join} from 'node:path';
const release=JSON.parse(readFileSync('packaging/release.json'));
const keyInfo=JSON.parse(readFileSync('packaging/update-key.json'));
if(!process.env.ROUTELOOM_UPDATE_PRIVATE_KEY) throw new Error('Set the ROUTELOOM_UPDATE_PRIVATE_KEY repository secret before publishing updates');
const key=createPrivateKey({key:Buffer.from(process.env.ROUTELOOM_UPDATE_PRIVATE_KEY,'base64'),format:'der',type:'pkcs8'});
if(Buffer.from(createPublicKey(key).export({format:'jwk'}).x,'base64url').toString('base64')!==keyInfo.publicKey)throw new Error('Wrong update signing key');
const dir=process.argv[2]??'.local/release-assets';
const files={windows:'routeloom-windows-x64.zip',linux:'routeloom-linux-amd64.deb',android:'routeloom-android.apk',macos:'routeloom-apple-signing-kit.zip',ios:'routeloom-apple-signing-kit.zip'};
const assets=Object.fromEntries(Object.entries(files).map(([platform,name])=>{
 const file=join(dir,name);
 return [platform,{name,size:statSync(file).size,sha256:createHash('sha256').update(readFileSync(file)).digest('hex')}];
}));
if(!Number.isSafeInteger(release.sequence)||release.sequence<2)throw new Error('Monotonic release.sequence required');
const payload=Buffer.from(JSON.stringify({schema:1,sequence:release.sequence,tag:release.tag,published:new Date().toISOString(),assets}));
const signature=sign(null,payload,key);
if(!verify(null,payload,createPublicKey(key),signature))throw new Error('Signing self-check failed');
writeFileSync(join(dir,'update.json'),JSON.stringify({keyId:keyInfo.id,payload:payload.toString('base64'),signature:signature.toString('base64')}));
