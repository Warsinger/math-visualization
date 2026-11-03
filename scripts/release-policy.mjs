import {execFileSync} from 'node:child_process';
import {appendFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
export function releaseIdentity(version, ref) {
  const match=/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(version);
  if(!match || match.slice(1,4).some(n=>Number(n)>65535) || match[4]?.split('.').some(p=>/^\d+$/.test(p)&&p.length>1&&p[0]==='0')) throw Error('Use SemVer without build metadata; numeric components must fit Windows 16-bit fields.');
  const tagged=ref.startsWith('refs/tags/');
  if(tagged && ref!=='refs/tags/v'+version) throw Error('Tag must exactly match v'+version+' from Cargo.toml');
  return {version,tagged,prerelease:!!match[4]};
}
export function verifyForwardVersion(base, next) {
  releaseIdentity(base,''); releaseIdentity(next,'');
  const parts=v=>v.split('-')[0].split('.').map(Number);
  const [a,b,c]=parts(base),[x,y,z]=parts(next);
  if(x<a || (x===a && (y<b || (y===b && z<=c)))) throw Error('Issue PR must advance the version beyond main; reconcile the version before merging.');
  if(x===a && y>b && z!==0) throw Error('Feature/minor increments must reset patch to zero.');
  if(x===a && y===b && z!==c+1) throw Error('Bug fix increments patch by one.');
}
export async function ensureUnpublished(repo,tag,token,request=fetch) {
  const response=await request('https://api.github.com/repos/'+repo+'/releases/tags/'+encodeURIComponent(tag),{headers:{Authorization:'Bearer '+token,Accept:'application/vnd.github+json'}});
  if(response.status===404)return;
  if(response.ok)throw Error('A release already exists for '+tag+'; choose a new version.');
  throw Error('Cannot verify release uniqueness: HTTP '+response.status);
}
async function main(){
  const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--no-deps','--format-version','1'],{encoding:'utf8'}));
  const {version}=metadata.packages.find(p=>p.name==='math-playground');
  const info=releaseIdentity(version,process.env.GITHUB_REF||'');
  if(process.env.GITHUB_EVENT_NAME==='pull_request') {
    const base=execFileSync('git',['show','origin/main:Cargo.toml'],{encoding:'utf8'}).match(/^version = "([^"]+)"/m)?.[1];
    verifyForwardVersion(base,version);
  }
  if(info.tagged){
    execFileSync('git',['diff','--exit-code']);
    execFileSync('git',['merge-base','--is-ancestor','HEAD','origin/main']);
    await ensureUnpublished(process.env.GITHUB_REPOSITORY,'v'+version,process.env.GITHUB_TOKEN);
  }
  if(process.env.GITHUB_OUTPUT)appendFileSync(process.env.GITHUB_OUTPUT,'version='+version+'\nprerelease='+info.prerelease+'\n');
  const message='Version '+version+'; '+(info.tagged?'validated release tag (publication remains manual)':'ordinary build with embedded commit provenance');
  console.log(message);
  if(process.env.GITHUB_STEP_SUMMARY)appendFileSync(process.env.GITHUB_STEP_SUMMARY,message+'\n');
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await main();
