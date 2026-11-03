import test from 'node:test';
import assert from 'node:assert/strict';
import {releaseIdentity,ensureUnpublished,verifyForwardVersion,compareVersions,verifyPullRequestVersion,documentationOnly} from './release-policy.mjs';
test('release tags match authoritative SemVer including prereleases',()=>{
 assert.equal(releaseIdentity('0.1.0','refs/tags/v0.1.0').tagged,true);
 assert.equal(releaseIdentity('0.2.0-rc.1','refs/tags/v0.2.0-rc.1').prerelease,true);
 assert.equal(releaseIdentity('0.1.0','refs/pull/1/merge').tagged,false);
 for(const [v,r] of [['0.1.0','refs/tags/v0.2.0'],['01.0.0',''],['1.0.0-01',''],['65536.0.0',''],['1.0.0+meta','']])assert.throws(()=>releaseIdentity(v,r));
});
test('existing releases and failed API access never pass uniqueness gate',async()=>{
 await ensureUnpublished('repo','v1','token',async()=>({status:404}));
 await assert.rejects(ensureUnpublished('repo','v1','token',async()=>({status:200,ok:true})),/already exists/);
 await assert.rejects(ensureUnpublished('repo','v1','token',async()=>({status:403})),/Cannot verify/);
});

test('issue versions advance, including stacked feature and bug-fix PRs',()=>{
 verifyForwardVersion('0.1.2','0.1.3');
 verifyForwardVersion('0.1.2','0.2.0');
 verifyForwardVersion('0.1.2','0.3.0'); // stacked on the 0.2.0 feature
 verifyForwardVersion('0.1.2','0.2.1'); // bug fix stacked on the 0.2.0 feature
 for(const [base,next] of [['0.2.0','0.2.0'],['0.2.0','0.1.1']])assert.throws(()=>verifyForwardVersion(base,next));
});

test('SemVer prerelease precedence supports candidate progression and stable promotion',()=>{
 const chain=['1.0.0-alpha','1.0.0-alpha.1','1.0.0-alpha.beta','1.0.0-beta','1.0.0-beta.2','1.0.0-beta.11','1.0.0-rc.1','1.0.0-rc.2','1.0.0'];
 for(let i=0;i<chain.length;i++) {
   assert.equal(compareVersions(chain[i],chain[i]),0);
   assert.throws(()=>verifyForwardVersion(chain[i],chain[i]));
   for(let j=i+1;j<chain.length;j++) {verifyForwardVersion(chain[i],chain[j]);assert.throws(()=>verifyForwardVersion(chain[j],chain[i]));}
 }
 verifyForwardVersion('0.3.0-rc.1','0.3.0-rc.2');verifyForwardVersion('0.3.0-rc.2','0.3.0');
 verifyForwardVersion('0.3.0-rc.999999999999999999999','0.3.0-rc.1000000000000000000000');
});
test('only a nonempty allowlisted documentation diff may retain the version',()=>{
 for(const paths of [['README.md'],['docs/UI-GUIDE.md','docs/images/example.png'],['SPEC.md','TESTING.md']]) {
   assert.equal(documentationOnly(paths),true);verifyPullRequestVersion('0.2.0','0.2.0',paths);
 }
 for(const paths of [[],['src/main.rs'],['README.md','src/main.rs'],['.github/workflows/ci.yml'],['Cargo.toml'],['AGENTS.md'],['docs/tool.mjs'],['docs/../src/main.rs'],['src/deleted.rs','docs/deleted.rs.md']]) {
   assert.equal(documentationOnly(paths),false);assert.throws(()=>verifyPullRequestVersion('0.2.0','0.2.0',paths));
   verifyPullRequestVersion('0.2.0','0.2.1',paths);
 }
 assert.throws(()=>verifyPullRequestVersion('0.2.0','0.1.9',['README.md']));
});
