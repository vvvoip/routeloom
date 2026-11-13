import {readFileSync,appendFileSync} from 'node:fs';
const {tag}=JSON.parse(readFileSync('packaging/release.json'));
const response=await fetch('https://api.github.com/repos/vvvoip/routeloom/releases/tags/'+encodeURIComponent(tag),{headers:{Accept:'application/vnd.github+json',Authorization:'Bearer '+process.env.GH_TOKEN,'User-Agent':'RouteLoom-site'}});
let ready=false;
if(response.status!==404){
 if(!response.ok)throw Error('Release lookup failed: '+response.status);
 const release=await response.json();
 const names=new Set(release.assets.map(a=>a.name));
 ready=!release.draft&&['routeloom-windows-x64.zip','routeloom-linux-amd64.deb','routeloom-android.apk','routeloom-apple-signing-kit.zip','update.json'].every(n=>names.has(n));
}
appendFileSync(process.env.GITHUB_OUTPUT,'ready='+ready+'\n');
console.log(ready?'Release assets exist; website may publish.':'Release assets are not ready; keeping the current website until the client workflow completes.');
