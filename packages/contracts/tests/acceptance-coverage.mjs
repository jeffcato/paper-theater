import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateAcceptanceCoverageLedgerV1} from '@paper-theater/contracts';
const output=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','acceptance-coverage-fixtures'],{encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(output.status,0,output.stderr);
const records=JSON.parse(output.stdout);
for(const r of records){let passed=true;try{await validateAcceptanceCoverageLedgerV1(r.ledger,new TextEncoder().encode(r.report_bytes),r.expected_checker);}catch{passed=false;}assert.equal(passed,r.valid,r.id);}
const report=JSON.parse(records[0].report_bytes);
assert.deepEqual(records[0].ledger.rows.map(r=>r.result),report.checks);
// Diagnostic provenance may make the aggregate Fail; retain original Uncertain observations.
const outcomes=new Set([...report.checks.map(r=>r.outcome),...report.observations.map(o=>o.claimed_outcome)]);
for(const outcome of ['missing','uncertain','fail'])assert(outcomes.has(outcome),outcome);
assert(records[0].ledger.blockers.includes('production-verifier-pending'));
console.log(`Acceptance coverage: ${records.length} Rust-produced exact-report cases agree; adverse outcomes and production gates retained`);
