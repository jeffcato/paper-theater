import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateProviderNativeExchangeV1} from '@paper-theater/contracts';
const output=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','provider-native-fixtures'],{encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(output.status,0,output.stderr);
const records=JSON.parse(output.stdout),encoder=new TextEncoder();
for(const c of records){const bytes={issued_operation:encoder.encode(c.issued_bytes),logical_request:encoder.encode(c.logical_bytes),wire_request:encoder.encode(c.wire_bytes),raw_response:c.raw_count?new Uint8Array(c.raw_count).fill(120):null};const original=structuredClone({operation:c.operation,result:c.result});let error;try{await validateProviderNativeExchangeV1(c.record,c.policy,c.operation,c.result,bytes);}catch(e){error=e;}assert.equal(!error,c.valid,c.id);if(error){assert.equal(error.code,c.rust_error.code,c.id);assert(error.field,c.id);}assert.deepEqual({operation:c.operation,result:c.result},original,c.id+' mutated original identities/accounting');}
console.log(`Native exchange: ${records.length} Rust-produced byte/identity/cap cases agree; uncertain accounting and dispatch gates retained`);
