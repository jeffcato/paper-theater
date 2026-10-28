import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validatePilotReviewPolicyV1,validatePilotReviewObservationV1,validatePilotPlanStructureV1,validateAcceptanceVerificationReport} from '@paper-theater/contracts';
const output=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','pilot-review-fixtures'],{encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(output.status,0,output.stderr);const cases=JSON.parse(output.stdout);
for(const c of cases){let failed=false;try{switch(c.kind){case 'policy':validatePilotReviewPolicyV1(c.value);break;case 'observation':validatePilotReviewObservationV1(c.value);break;case 'structure':validatePilotPlanStructureV1(c.value);break;case 'report':validateAcceptanceVerificationReport(c.value,c.expected_checker??c.value.checker);break;default:throw new Error(c.kind);}}catch{failed=true;}assert.equal(!failed,c.valid,c.id);}
console.log(`Pilot review: ${cases.length} Rust-produced policy/structure/recorded/report cases agree; listening/production stay pending`);
