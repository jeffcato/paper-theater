import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {authenticateTrustedLoad,sha256,verifyAcceptedPublication} from '@paper-theater/contracts';
const enc=new TextEncoder();
const out=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','publication-decision-fixtures'],{encoding:'utf8',maxBuffer:16*1024*1024});
assert.equal(out.status,0,out.stderr);
const {diagnostic,chapter}=JSON.parse(out.stdout);
for(const [name,b] of [['diagnostic',diagnostic],['chapter',chapter]]){
 const allowlist=JSON.parse(b.allowlist_json);
 const files=b.files.map(f=>({path:f.path,data:enc.encode(f.text)}));
 const got=await verifyAcceptedPublication(b.ledger,enc.encode(b.report),b.checker,allowlist,b.catalog_sha256,files,[b.private_digest]);
 assert.deepEqual(got,b.expected,name+': TypeScript record differs from Rust');
 b.allowlist=JSON.parse(b.allowlist_json);
}
const b=diagnostic;
const files=()=>b.files.map(f=>({path:f.path,data:enc.encode(f.text)}));
await assert.rejects(verifyAcceptedPublication(b.ledger,enc.encode(b.report+' '),b.checker,b.allowlist,b.catalog_sha256,files(),[b.private_digest]),'forged report accepted');
const changed=files();changed[1].data[1]^=1;
await assert.rejects(verifyAcceptedPublication(b.ledger,enc.encode(b.report),b.checker,b.allowlist,b.catalog_sha256,changed,[b.private_digest]),'changed payload accepted');
await assert.rejects(verifyAcceptedPublication(b.ledger,enc.encode(b.report),b.checker,chapter.allowlist,chapter.files.map(f=>({path:f.path,data:enc.encode(f.text)})),[b.private_digest]),'stale cross-execution join accepted');
const short={...b.ledger,rows:b.ledger.rows.slice(0,-1)};
await assert.rejects(verifyAcceptedPublication(short,enc.encode(b.report),b.checker,b.allowlist,b.catalog_sha256,files(),[b.private_digest]),'incomplete coverage accepted');
await assert.rejects(verifyAcceptedPublication(b.ledger,enc.encode(b.report),b.checker,b.allowlist,b.catalog_sha256,files().slice(0,2),[b.private_digest]),'partial payload accepted');
await assert.rejects(verifyAcceptedPublication(b.ledger,enc.encode(b.report),b.checker,b.allowlist,await sha256(enc.encode('elsewhere')),files(),[b.private_digest]),'absent catalog accepted');
const load={schema_version:1,profile_id:'paper-player-r186-v1',checker_sha256:b.checker.sha256,acceptance_report_sha256:b.ledger.report_sha256,catalog_entry_sha256:b.catalog_sha256,release_id:'oz-1',allow_diagnostic:true};
await authenticateTrustedLoad(load,b.ledger,b.catalog_sha256);
await assert.rejects(authenticateTrustedLoad({...load,acceptance_report_sha256:'0'.repeat(64)},b.ledger,b.catalog_sha256),'forged load report accepted');
await assert.rejects(authenticateTrustedLoad({...load,catalog_entry_sha256:'0'.repeat(64)},b.ledger,b.catalog_sha256),'forged load catalog accepted');
console.log('Publication decision: 2 exact Rust/TypeScript records agree; forged, changed, stale, incomplete, partial and absent-catalog cases rejected; trusted loads authenticate');
