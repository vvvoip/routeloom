import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
test('release version, update sequence and pinned key agree',async()=>{
 const release=JSON.parse(await readFile('packaging/release.json','utf8'));
 const pubspec=await readFile('apps/client/pubspec.yaml','utf8');
 const updates=await readFile('apps/client/lib/services/updates.dart','utf8');
 const key=JSON.parse(await readFile('packaging/update-key.json','utf8'));
 const dartKey=await readFile('apps/client/lib/services/update_key.dart','utf8');
 assert.ok(Number.isSafeInteger(release.sequence)&&release.sequence>=2);
 assert.ok(pubspec.includes('version: '+release.version+'+'+release.sequence));
 assert.ok(updates.includes('const currentReleaseSequence = '+release.sequence+';'));
 assert.ok(updates.includes("const currentReleaseVersion = '"+release.version+"';"));
 assert.ok(release.tag.startsWith('v'+release.version+'-'));
 assert.ok(dartKey.includes(key.publicKey));
});
