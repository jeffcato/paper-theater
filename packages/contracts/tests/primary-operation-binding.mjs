import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validatePrimaryProvenanceV1} from '@paper-theater/contracts';
const out=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','primary-operation-fixtures'],{encoding:'utf8',maxBuffer:4*1024*1024});assert.equal(out.status,0,out.stderr);
const rows=JSON.parse(out.stdout),encoder=new TextEncoder();
for(const row of rows){let error;try{await validatePrimaryProvenanceV1(row.provenance,row.operation,row.result,encoder.encode(row.primary_text));}catch(e){error=e;}assert.equal(!error,row.valid,row.id+': '+error?.message);}
console.log(`Primary operation binding: ${rows.length} exact-byte Rust/TypeScript cases agree; forged and stale bindings stay rejected`);
