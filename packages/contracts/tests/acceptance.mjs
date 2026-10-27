import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {acceptanceCheckMatrix,validateAcceptanceVerificationReport,productionBlockers} from '@paper-theater/contracts';
const processResult=spawnSync('cargo',['run','--offline','--quiet','-p','pt-contracts','--example','acceptance-fixtures'],{encoding:'utf8',maxBuffer:8*1024*1024});assert.equal(processResult.status,0,processResult.stderr);
const data=JSON.parse(processResult.stdout);assert.deepEqual(acceptanceCheckMatrix(),data.matrix);
for(const c of data.cases){let error;try{validateAcceptanceVerificationReport(c.report,data.checker);}catch(e){error=e;}assert.equal(!error,c.valid,c.id+': '+error?.message);if(error)assert(error.code&&error.field);else{assert.equal(c.report.production_approved,false);assert(c.report.blockers.includes('production-verifier-pending'));}}
console.log(`acceptance: ${data.cases.length} Rust-produced private reports agree; unimplemented verifiers remain blocked`);
