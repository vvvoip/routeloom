import {createPrivateKey,createPublicKey} from 'node:crypto';
import {readFileSync} from 'node:fs';
for(const name of ['ROUTELOOM_UPDATE_PRIVATE_KEY','ROUTELOOM_ANDROID_KEYSTORE_BASE64','ROUTELOOM_ANDROID_KEY_PASSWORD'])if(!process.env[name]?.trim())throw Error('Missing repository secret: '+name);
const pinned=JSON.parse(readFileSync('packaging/update-key.json'));
try{const key=createPrivateKey({key:Buffer.from(process.env.ROUTELOOM_UPDATE_PRIVATE_KEY.trim(),'base64'),format:'der',type:'pkcs8'});if(Buffer.from(createPublicKey(key).export({format:'jwk'}).x,'base64url').toString('base64')!==pinned.publicKey)throw Error('key');}catch{throw Error('The update signing secret does not match the pinned publisher key');}
if(Buffer.from(process.env.ROUTELOOM_ANDROID_KEYSTORE_BASE64.trim(),'base64').length<1000)throw Error('Android keystore secret is incomplete');
console.log('Update signing key matches the pinned publisher key; Android signing inputs are present.');
