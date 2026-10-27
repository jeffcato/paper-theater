import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import * as api from '@paper-theater/contracts';
assert.equal(typeof api.providerServicesV2,'function','old TS implementors need explicit unavailable defaults');
let legacy=0,other=0;
const old={provider:async()=>{legacy++;throw Error('legacy dispatch');},check_cancelled:async()=>{other++;}};
const services=api.providerServicesV2(old);
const result=spawnSync('cargo',['run','--offline','--quiet','-p','pt-contracts','--example','provider-service-fixtures'],{encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(result.status,0,result.stderr);
const records=JSON.parse(result.stdout);
for(const {case:c,rust_error,roundtrip} of records){
 let error;
 try{
  if(c.kind==='proposal')api.validateProviderProposalV2(c.wire);
  else if(c.kind==='observer')api.validateProviderObserverV2(c.wire);
  else if(c.kind==='issued')api.validateProviderIssuedOperationV2(c.wire,c.proposal,c.observer);
  else if(c.kind==='recovery_request')api.validateProviderRecoveryRequestV2(c.wire);
  else api.validateProviderRecoveryResultV2(c.wire,c.request,c.observer);
 }catch(e){error=e;}
 assert.equal(!error,c.valid,c.id+': '+error?.message);
 if(error){assert(error.code&&error.field);assert.equal(error.code,rust_error.code,c.id);}
 else{
  // Check Rust-normalized records as well as original nullable/omitted wire input.
  if(c.kind==='proposal')api.validateProviderProposalV2(roundtrip);
  else if(c.kind==='recovery_result')api.validateProviderRecoveryResultV2(roundtrip,c.request,c.observer);
 }
}
await assert.rejects(services.issue_provider_v2(records[0].case.wire),{code:'provider_v2_unavailable'});
await assert.rejects(services.execute_provider_v2(records[1].case.wire),{code:'provider_v2_unavailable'});
await assert.rejects(services.recover_provider_v2(records[3].case.wire),{code:'provider_v2_unavailable'});
assert.equal(legacy,0);assert.equal(other,0);
const capable={...old,tag:'bound',async issue_provider_v2(){assert.equal(this.tag,'bound');return records[1].case.wire;}};
assert.deepEqual(await api.providerServicesV2(capable).issue_provider_v2(records[0].case.wire),records[1].case.wire);
await assert.rejects(api.providerServicesV2(capable).execute_provider_v2(records[1].case.wire),{code:'provider_v2_unavailable'});
console.log(`provider service: ${records.length} Rust-produced proposal/issued/observer/recovery cases; unavailable defaults never call legacy`);

// Errors from a supplied capability propagate; they never trigger legacy fallback.
for(const mode of ['fixture','replay']){
 let guarded=0;
 const refuse=async()=>{guarded++;throw Object.assign(Error(mode+' forbids dispatch'),{code:'offline_dispatch'});};
 const offline=api.providerServicesV2({...old,issue_provider_v2:refuse,execute_provider_v2:refuse,recover_provider_v2:refuse});
 await assert.rejects(offline.issue_provider_v2(records[0].case.wire),{code:'offline_dispatch'});
 await assert.rejects(offline.execute_provider_v2(records[1].case.wire),{code:'offline_dispatch'});
 await assert.rejects(offline.recover_provider_v2(records[3].case.wire),{code:'offline_dispatch'});
 assert.equal(guarded,3);assert.equal(legacy,0);
}
