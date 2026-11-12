import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, access} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve('website/dist');
test('English and Chinese have the same translation structure',async()=>{
 const en=JSON.parse(await readFile('website/locales/en.json','utf8'));
 const zh=JSON.parse(await readFile('website/locales/zh-CN.json','utf8'));
 assert.deepEqual(Object.keys(en).sort(),Object.keys(zh).sort());
 for(const key of Object.keys(en)) if(Array.isArray(en[key])) assert.equal(en[key].length,zh[key].length,key);
});
for(const page of ['index.html','en/index.html','zh-CN/index.html','en/docs/index.html','zh-CN/docs/index.html']) {
 test('Page links and assets resolve: '+page,async()=>{
  const html=await readFile(path.join(root,page),'utf8');
  assert.match(html,/id="main"/);
  assert.match(html,/lang="(en|zh-CN)"/);
  assert.doesNotMatch(html,/undefined|\[object Object\]/);
  for(const match of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
   const url=match[1];
   if(!url.startsWith('/routeloom/') && !url.startsWith('#')) continue;
   const [relative,fragment]=url.split('#');
   let target=relative ? path.join(root,relative.slice('/routeloom/'.length)) : path.join(root,page);
   if(relative.endsWith('/')) target=path.join(target,'index.html');
   await access(target);
   if(fragment) assert.ok((await readFile(target,'utf8')).includes('id="'+fragment+'"'),url);
  }
 });
}
