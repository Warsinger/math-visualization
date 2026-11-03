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
export function compareVersions(left, right) {
  releaseIdentity(left,''); releaseIdentity(right,'');
  const parse=v=>{const split=v.indexOf('-');return {core:(split<0?v:v.slice(0,split)).split('.').map(Number),pre:split<0?[]:v.slice(split+1).split('.')};};
  const a=parse(left),b=parse(right);
  for(let i=0;i<3;i++)if(a.core[i]!==b.core[i])return a.core[i]<b.core[i]?-1:1;
  if(!a.pre.length || !b.pre.length)return a.pre.length===b.pre.length?0:a.pre.length?-1:1;
  for(let i=0;i<Math.max(a.pre.length,b.pre.length);i++) {
    if(i>=a.pre.length)return -1;if(i>=b.pre.length)return 1;
    const x=a.pre[i],y=b.pre[i];if(x===y)continue;
    const xn=/^\d+$/.test(x),yn=/^\d+$/.test(y);
    if(xn&&yn)return BigInt(x)<BigInt(y)?-1:1;
    if(xn!==yn)return xn?-1:1;
    return x<y?-1:1;
  }
  return 0;
}
export function verifyForwardVersion(base, next) {
  if(compareVersions(next,base)<=0)throw Error('Issue PR must advance the version beyond main; reconcile the version before merging.');
}
export function documentationOnly(paths) {
  return paths.length>0 && paths.every(path=>
    !path.split('/').some(part=>!part || part==='.' || part==='..') && (['README.md','TESTING.md','SPEC.md'].includes(path) ||
    /^docs\/.+\.(?:md|rst|txt|png|jpe?g|gif|svg|webp|pdf)$/.test(path)));
}
export function verifyPullRequestVersion(base, next, paths) {
  if(compareVersions(next,base)===0 && documentationOnly(paths))return;
  verifyForwardVersion(base,next);
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
    // --no-renames includes both old/deleted and new paths; moving code into docs is not an exemption.
    const paths=execFileSync('git',['diff','--no-renames','--name-only','-z','origin/main...HEAD'],{encoding:'utf8'}).split('\0').filter(Boolean);
    verifyPullRequestVersion(base,version,paths);
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
