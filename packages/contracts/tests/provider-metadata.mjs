import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import * as api from '@paper-theater/contracts';
assert.equal(typeof api.validateProviderOperationV2,'function');
const result=spawnSync('cargo',['run','--offline','--quiet','-p','pt-contracts','--example','provider-fixtures'],{encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(result.status,0,result.stderr);
const records=JSON.parse(result.stdout);
for(const {case:c,rust_error,roundtrip,scope_keys} of records){
 let failure;
 try{if(c.kind==='operation')api.validateProviderOperationV2(c.wire);else if(c.kind==='result')api.validateProviderResultV2(c.wire,c.operation);else if(c.kind==='legacy_operation')api.validateLegacyProviderOperation(c.wire);else api.validateLegacyProviderResult(c.wire);}catch(e){failure=e;}
 assert.equal(!failure,c.valid,c.id+': '+failure?.message);
 if(failure){assert(failure.code&&failure.field,c.id+' needs a typed field/error');if(c.code)assert.equal(failure.code,rust_error.code,c.id);}
 else if(c.kind==='operation')assert.deepEqual(await Promise.all(c.wire.quota_windows.map(w=>api.providerQuotaScopeKey(w.scope))),scope_keys,c.id);
 else if(c.kind.startsWith('legacy_'))assert.deepEqual(roundtrip,c.expected_roundtrip??c.wire);
}
const key=async id=>api.providerQuotaScopeKey(records.find(r=>r.case.id===id).case.wire.quota_windows[0].scope);
assert.equal(await key('free-money-request'),await key('second-credential-shares-project-scope'));
assert.notEqual(await key('scope-tuple-a'),await key('scope-tuple-b'));
console.log(`provider metadata: ${records.length} Rust-produced operation/result/legacy cases agree in TypeScript`);
