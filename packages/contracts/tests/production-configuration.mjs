import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateProductionConfigurationV1,validateConfiguredProviderProposalV1} from '@paper-theater/contracts';
const output=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','production-configuration-fixtures'],{encoding:'utf8',maxBuffer:1024*1024});
assert.equal(output.status,0,output.stderr);
const records=JSON.parse(output.stdout);
for(const row of records.configurations){let passed=true;try{validateProductionConfigurationV1(row.value);}catch{passed=false;}assert.equal(passed,row.valid,row.id);}
for(const row of records.proposals){let passed=true;try{validateConfiguredProviderProposalV1(row.configuration,row.scope_id,row.input,row.proposal);}catch{passed=false;}assert.equal(passed,row.valid,row.id);}
console.log(`Production configuration: ${records.configurations.length+records.proposals.length} Rust-produced cases agree; wire input grants no authority`);
