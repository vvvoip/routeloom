import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
test('both client launch languages translate every UI key',async()=>{
 const en=JSON.parse(await readFile('apps/client/lib/l10n/app_en.arb','utf8'));
 const zh=JSON.parse(await readFile('apps/client/lib/l10n/app_zh.arb','utf8'));
 const keys=x=>Object.keys(x).filter(k=>!k.startsWith('@')).sort();
 assert.deepEqual(keys(en),keys(zh));
 for(const key of keys(en)) assert.ok(typeof zh[key]==='string'&&zh[key].trim().length>0,key);
 const region=JSON.parse(await readFile('apps/client/lib/l10n/app_zh_CN.arb','utf8'));
 assert.equal(region['@@locale'],'zh_CN');
 assert.equal(zh['@@locale'],'zh');
});
