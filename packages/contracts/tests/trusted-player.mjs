import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateTrustedPlayerLoadV1} from '@paper-theater/contracts';
const out=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','trusted-player-fixtures'],{encoding:'utf8',maxBuffer:4*1024*1024});assert.equal(out.status,0,out.stderr);
const rows=JSON.parse(out.stdout);
for(const row of rows){let error;try{validateTrustedPlayerLoadV1(row.load);}catch(e){error=e;}assert.equal(!error,row.valid,row.id+': '+error?.message);}
console.log(`Trusted player load: ${rows.length} exact-byte Rust/TypeScript cases agree; placeholders stay rejected`);
