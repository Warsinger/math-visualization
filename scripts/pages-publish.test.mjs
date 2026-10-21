import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const script=new URL('./pages.mjs',import.meta.url).href;
const sha='a'.repeat(40);
function fixture(paused=false) {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'pages-handoff-'));
  const env={...process.env,GITHUB_ACTIONS:'true',PAGES_SERIALIZED:'true',GITHUB_SHA:sha,GITHUB_REF:'refs/heads/feature',GITHUB_REF_NAME:'feature',GITHUB_RUN_ID:'123',GITHUB_RUN_ATTEMPT:'1',CLOUDFLARE_ACCOUNT_ID:'a'.repeat(32),CLOUDFLARE_API_TOKEN:'test-only',GITHUB_TOKEN:'test-only',GITHUB_OUTPUT:path.join(dir,'outputs'),PAGES_HANDOFF:path.join(dir,'handoff.json')};
  const run=(mode,closed=false)=>spawnSync(process.execPath,['--input-type=module','-e',`
    import fs from 'node:fs';
    let deleted=false;
    globalThis.fetch=async(url,options={})=>{
      const pathname=new URL(url).pathname;
      if(pathname.includes('api-missing'))throw Error('Unexpected route');
      if(url.startsWith('https://api.cloudflare.com/')){
        if(options.method==='DELETE'){deleted=true;fs.writeFileSync(${JSON.stringify(path.join(dir,'deleted'))},pathname);return Response.json({success:true});}
        if(pathname.endsWith('/deployments')){
          const marker=fs.existsSync(process.env.PAGES_HANDOFF)?JSON.parse(fs.readFileSync(process.env.PAGES_HANDOFF)).marker:'';
          return Response.json({success:true,result:marker&&!deleted?[{id:'uploaded',environment:'preview',url:'https://abc.math-viz.pages.dev',deployment_trigger:{metadata:{commit_message:marker}}}]:[],result_info:{total_pages:1}});
        }
        return Response.json({success:true,result:{subdomain:'math-viz.pages.dev',production_branch:'main',deployment_configs:{production:{env_vars:{MP_PUBLISH_STATE:{value:JSON.stringify({version:1,paused:${paused},barrier:0,previews:{}})}}}}}});
      }
      if(pathname.endsWith('/commits')||pathname.endsWith('/events')||pathname.endsWith('/comments'))return Response.json([]);
      if(pathname.includes('/actions/runs/'))return Response.json({created_at:'2026-10-21T10:00:00Z',head_branch:'feature'});
      if(pathname.endsWith('/pulls/9'))return Response.json({number:9,state:${closed}?'closed':'open',base:{ref:'main',repo:{full_name:'Warsinger/math-visualization'}},head:{sha:${JSON.stringify(sha)},ref:'feature',repo:{full_name:'Warsinger/math-visualization'}}});
      throw Error('Unexpected route '+pathname);
    };
    process.argv=['node','pages.mjs',${JSON.stringify(mode)},'--target','preview','--pr','9','--apply'];await import(${JSON.stringify(script)});
  `],{env,encoding:'utf8'});
  return {dir,env,run,cleanup:()=>fs.rmSync(dir,{recursive:true,force:true})};
}
test('preflight prepares metadata without uploading; paused runs expose no upload command',()=>{
  for(const paused of [false,true]){
    const f=fixture(paused);try{
      const result=f.run('prepare-publish');assert.equal(result.status,0,result.stderr);
      if(paused)assert.equal(fs.existsSync(f.env.GITHUB_OUTPUT),false);
      else {
        const outputs=fs.readFileSync(f.env.GITHUB_OUTPUT,'utf8');assert.match(outputs,/^publish=true\ncommand=pages deploy /);
        assert.ok(outputs.includes('--commit-message'));const handoff=JSON.parse(fs.readFileSync(f.env.PAGES_HANDOFF));assert.equal(handoff.sha,sha);assert.equal(JSON.parse(handoff.marker.slice(4)).pr,9);
      }
    }finally{f.cleanup();}
  }
});
test('finish removes the upload if the PR closes between guard and upload',()=>{
  const f=fixture();try{
    assert.equal(f.run('prepare-publish').status,0);
    const result=f.run('finish-publish',true);assert.equal(result.status,0,result.stderr);
    assert.match(fs.readFileSync(path.join(f.dir,'deleted'),'utf8'),/deployments\/uploaded$/);
  }finally{f.cleanup();}
});
test('finish rejects a handoff from another workflow run',()=>{
  const f=fixture();try{
    assert.equal(f.run('prepare-publish').status,0);
    const handoff=JSON.parse(fs.readFileSync(f.env.PAGES_HANDOFF));handoff.run='999';fs.writeFileSync(f.env.PAGES_HANDOFF,JSON.stringify(handoff));
    const result=f.run('finish-publish');assert.notEqual(result.status,0);assert.match(result.stderr,/Invalid publish handoff/);
  }finally{f.cleanup();}
});
