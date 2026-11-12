import {readFile, mkdir, writeFile, cp} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {renderPage} from './render.mjs';
const directory=path.dirname(fileURLToPath(import.meta.url));
const base=process.env.SITE_BASE ?? '/routeloom';
if (!/^\/[a-z0-9/-]*$/i.test(base) || base.endsWith('/')) throw Error('SITE_BASE must be a URL path without a trailing slash');
const output=path.join(directory,'dist');
await mkdir(output,{recursive:true});
for(const lang of ['en','zh-CN']) {
 const t=JSON.parse(await readFile(path.join(directory,'locales',lang+'.json'),'utf8'));
 for(const docs of [false,true]) {
  const dest=path.join(output,lang,docs?'docs':'');
  await mkdir(dest,{recursive:true});
  await writeFile(path.join(dest,'index.html'),renderPage(t,lang,base,docs));
 }
 if(lang==='en') await writeFile(path.join(output,'index.html'),renderPage(t,lang,base));
}
await cp(path.join(directory,'assets'),path.join(output,'assets'),{recursive:true});
await writeFile(path.join(output,'.nojekyll'),'');
await writeFile(path.join(output,'404.html'),'<!doctype html><html lang="en"><meta charset="utf-8"><title>RouteLoom · 404</title><h1>Page not found / 页面不存在</h1><a href="'+base+'/en/">English</a> · <a href="'+base+'/zh-CN/">简体中文</a></html>');
console.log('Built bilingual RouteLoom website: '+output);
