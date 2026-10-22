// Keep the complete generated Rust runtime and shared frontend in one immutable release.
import {createHash} from 'node:crypto';
import {mkdir, readFile, rm, writeFile} from 'node:fs/promises';
import {dirname, resolve, sep} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';

export const runtimeNames = ['math_playground.js','math_playground_bg.wasm','app.mjs','styles.css','shared/module-runtime.mjs','shared/status.mjs','shared/canvas.css','shared/footer.css'];
export function runtimeHash(files) {
  const digest=createHash('sha256');
  for(const name of runtimeNames) digest.update(name).update('\0').update(files.get(name));
  return digest.digest('hex').slice(0,24);
}
export async function prepareWeb(root = fileURLToPath(new URL('../', import.meta.url))) {
  const output=resolve(root,'dist/web');
  const files=new Map();
  for(const name of runtimeNames) {
    const source=name.startsWith('shared/') ? resolve(root,'node_modules/@warsinger/web-shared/browser',name.slice(7)) : resolve(root,name.startsWith('math_playground')?'target/web-bindgen':'web',name);
    const data=await readFile(source);
    if(data.length>25*1024*1024) throw new Error(name+' exceeds the Pages 25 MiB asset limit.');
    files.set(name,data);
  }
  if(!files.get('app.mjs').toString().includes("import('./math_playground.js')")) throw new Error('Rust bootstrap import was not found.');
  const hash=runtimeHash(files),relative=`assets/${hash}`;
  let template=await readFile(resolve(root,'web/index.html'),'utf8');
  for(const name of ['app.mjs','styles.css']) {
    if(!template.includes(`"./${name}"`)) throw new Error(name+' entry reference was not found.');
    template=template.replaceAll(`"./${name}"`,`"./${relative}/${name}"`);
  }
  // Only clear the verified generated output directory inside this repository.
  if(!output.startsWith(resolve(root,'dist')+sep)) throw new Error('Invalid generated output path.');
  await rm(output,{recursive:true,force:true});
  for(const [name,data] of files){const target=resolve(output,relative,name);await mkdir(dirname(target),{recursive:true});await writeFile(target,data);}
  await writeFile(resolve(output,'index.html'),template);
  await writeFile(resolve(output,'_headers'),await readFile(resolve(root,'web/_headers')));
  for(const [source,destination] of [['favicon.ico','favicon.ico'],['app-192.png','icon-192.png'],['apple-touch-icon.png','apple-touch-icon.png']]) await writeFile(resolve(output,destination),await readFile(resolve(root,'assets/icon',source)));
  await writeFile(resolve(output,'404.html'),'<!doctype html><title>Not found</title><h1>Not found</h1>');
  console.log(`Prepared browser assets: ${relative}`);
  return {hash,relative,output};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href) await prepareWeb();
