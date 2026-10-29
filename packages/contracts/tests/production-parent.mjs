import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateProductionDerivationV1} from '@paper-theater/contracts';
const out=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','production-parent-fixtures'],{encoding:'utf8',maxBuffer:4*1024*1024});assert.equal(out.status,0,out.stderr);
const rows=JSON.parse(out.stdout),encoder=new TextEncoder();
for(const row of rows){let error;try{await validateProductionDerivationV1(row.policy,row.derived,encoder.encode(row.policy_bytes),encoder.encode(row.logical_bytes));}catch(e){error=e;}assert.equal(!error,row.valid,row.id+': '+error?.message);}
console.log(`Compiler parent policy: ${rows.length} exact-byte Rust/TypeScript cases agree; metadata grants no authority`);
