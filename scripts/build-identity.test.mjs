import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';

// Compile the actual build script in a tiny isolated crate: no GUI, no working-tree edits.
test('real Cargo rebuild refreshes identity after tracked docs/workflow edits and clean restoration', {timeout:180000}, t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'math-identity-'));
 t.after(()=>{assert.ok(path.basename(root).startsWith('math-identity-'));assert.equal(path.dirname(path.resolve(root)),path.resolve(os.tmpdir()));fs.rmSync(root,{recursive:true,force:true,maxRetries:3,retryDelay:100});});
 for(const dir of ['src','web','scripts','docs','assets/icon','.github/workflows'])fs.mkdirSync(path.join(root,dir),{recursive:true});
 const repository=new URL('../',import.meta.url);
 for(const name of ['build.rs','build_identity.rs','Cargo.lock','assets/icon/app.ico'])fs.copyFileSync(new URL(name,repository),path.join(root,name));
 fs.writeFileSync(path.join(root,'Cargo.toml'),'[package]\nname="identity-probe"\nversion="0.2.1"\nedition="2024"\n[build-dependencies]\nserde_json="1"\nwinresource="0.1.34"\n');
 fs.writeFileSync(path.join(root,'src/main.rs'),'fn main(){println!("{}",include_str!(concat!(env!("OUT_DIR"),"/build-info.json")));}');
 fs.writeFileSync(path.join(root,'.gitignore'),'target/\n');
 const files=['README.md','docs/DOWNLOADS.md','.github/workflows/example.yml','AGENTS.md'];
 for(const name of files)fs.writeFileSync(path.join(root,name),'original\n');
 const env={...process.env,CARGO_TARGET_DIR:path.join(root,'target'),GIT_CONFIG_NOSYSTEM:'1',GIT_CONFIG_GLOBAL:process.platform==='win32'?'NUL':'/dev/null'};
 for(const name of ['GITHUB_REF','GITHUB_RUN_ID','GITHUB_RUN_ATTEMPT','GITHUB_EVENT_NAME'])delete env[name];
 const run=(command,args)=>execFileSync(command,args,{cwd:root,env,encoding:'utf8',stdio:['ignore','pipe','pipe']});
 run('git',['init']);
 const commit=()=>{run('git',['add','.']);run('git',['-c','user.name=Identity Test','-c','user.email=identity-test@example.invalid','commit','-m','fixture']);};
 commit();
 const build=()=>{run('cargo',['+stable','build','--offline','--quiet']);return JSON.parse(run(path.join(root,'target/debug/identity-probe'+(process.platform==='win32'?'.exe':'')),[]));};
 // Resolve the fixture lock offline, then commit it to start from a genuinely clean tree.
 build();commit();
 let info=build();assert.equal(info.dirty,false);assert.match(info.display_version,/-dev.local\+/);
 for(const name of files){
   fs.appendFileSync(path.join(root,name),'changed\n');
   info=build();assert.equal(info.dirty,true,name);assert.match(info.display_version,/\.dirty$/);
   fs.writeFileSync(path.join(root,name),'original\n');
   assert.equal(build().dirty,false,'restored '+name);
 }
 fs.writeFileSync(path.join(root,'untracked.txt'),'not part of tracked-source identity');
 assert.equal(build().dirty,false,'untracked files explicitly excluded');
 fs.appendFileSync(path.join(root,'docs/DOWNLOADS.md'),'staged\n');run('git',['add','docs/DOWNLOADS.md']);
 assert.equal(build().dirty,true,'staged change');
 commit();const final=build();assert.equal(final.dirty,false);assert.notEqual(final.revision,info.revision);
 run('git',['pack-refs','--all']);build();
 const previous=JSON.parse(run(path.join(root,'target/debug/identity-probe'+(process.platform==='win32'?'.exe':'')),[])).revision;
 run('git',['-c','user.name=Identity Test','-c','user.email=identity-test@example.invalid','commit','--allow-empty','-m','move packed ref']);
 assert.notEqual(build().revision,previous,'packed-to-loose ref movement invalidates provenance');
 // No-op builds should retain the compiled metadata file, not rebuild on every invocation.
 const find=dir=>fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?find(path.join(dir,e.name)):e.name==='build-info.json'?[path.join(dir,e.name)]:[]);
 const metadata=find(path.join(root,'target/debug/build'))[0];
 const before=fs.statSync(metadata,{bigint:true}).mtimeNs;build();
 assert.equal(fs.statSync(metadata,{bigint:true}).mtimeNs,before,'no-op build does not rewrite metadata');
});
