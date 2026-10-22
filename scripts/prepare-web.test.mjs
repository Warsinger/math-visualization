import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {prepareWeb} from './prepare-web.mjs';
function fixture(t){
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'math-shared-web-'));
 t.after(()=>{assert.ok(path.basename(root).startsWith('math-shared-web-'));fs.rmSync(root,{recursive:true,force:true});});
 for(const dir of ['web','target/web-bindgen','assets/icon','node_modules/@warsinger/web-shared'])fs.mkdirSync(path.join(root,dir),{recursive:true});
 for(const file of ['index.html','app.mjs','styles.css','_headers'])fs.copyFileSync(new URL('../web/'+file,import.meta.url),path.join(root,'web',file));
 fs.cpSync(new URL('../node_modules/@warsinger/web-shared/browser',import.meta.url),path.join(root,'node_modules/@warsinger/web-shared/browser'),{recursive:true});
 for(const file of ['favicon.ico','app-192.png','apple-touch-icon.png'])fs.copyFileSync(new URL('../assets/icon/'+file,import.meta.url),path.join(root,'assets/icon',file));
 fs.writeFileSync(path.join(root,'target/web-bindgen/math_playground.js'),"export default async function init(){}; const url=new URL('math_playground_bg.wasm', import.meta.url);");
 fs.writeFileSync(path.join(root,'target/web-bindgen/math_playground_bg.wasm'),Buffer.from([0,97,115,109,1,0,0,0]));
 return root;
}
test('packages the real module/style tree with Rust glue and Wasm under one immutable URL',async t=>{
 const root=fixture(t),{relative,hash,output}=await prepareWeb(root);
 const html=fs.readFileSync(path.join(output,'index.html'),'utf8');
 assert.ok(html.includes(`src="./${relative}/app.mjs"`));assert.ok(html.includes(`href="./${relative}/styles.css"`));
 assert.match(html,/<canvas class="app-canvas" id="playground"/);assert.doesNotMatch(html,/<iframe/);
 for(const file of ['app.mjs','styles.css','shared/module-runtime.mjs','shared/status.mjs','shared/canvas.css','shared/footer.css','math_playground.js','math_playground_bg.wasm'])assert.ok(fs.existsSync(path.join(output,relative,file)),file);
 const {hash:same}=await prepareWeb(root);assert.equal(same,hash);
 fs.appendFileSync(path.join(root,'node_modules/@warsinger/web-shared/browser/footer.css'),'\n/* new shared style */');assert.notEqual((await prepareWeb(root)).hash,hash);
});
test('frontend-only changes get a fresh runtime path and oversized files do not replace a valid build',async t=>{
 const root=fixture(t),first=await prepareWeb(root);
 fs.appendFileSync(path.join(root,'web/app.mjs'),'\n// app bootstrap change');const next=await prepareWeb(root);assert.notEqual(next.hash,first.hash);
 const validHTML=fs.readFileSync(path.join(next.output,'index.html'),'utf8');
 fs.writeFileSync(path.join(root,'target/web-bindgen/math_playground_bg.wasm'),Buffer.alloc(25*1024*1024+1));await assert.rejects(prepareWeb(root),/25 MiB/);
 assert.equal(fs.readFileSync(path.join(next.output,'index.html'),'utf8'),validHTML);
});
