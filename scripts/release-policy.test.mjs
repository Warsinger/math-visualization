import test from 'node:test';
import assert from 'node:assert/strict';
import {releaseIdentity,ensureUnpublished,verifyForwardVersion} from './release-policy.mjs';
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
