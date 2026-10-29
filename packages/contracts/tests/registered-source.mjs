import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {validateAcceptanceVerificationReport,registeredSourceCatalogV1,REGISTERED_SOURCE_CATALOG_SHA256,acceptanceCheckMatrix,requiredAcceptanceChecks} from '@paper-theater/contracts';
const output=spawnSync('cargo',['run','--quiet','--locked','--offline','-p','pt-contracts','--example','registered-source-fixtures'],{encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(output.status,0,output.stderr);
const data=JSON.parse(output.stdout);assert.deepEqual(registeredSourceCatalogV1(),data.catalog);assert.equal(REGISTERED_SOURCE_CATALOG_SHA256,data.catalog_sha256);
for(const r of data.records){let error;try{validateAcceptanceVerificationReport(r.report,r.checker);}catch(e){error=e;}assert.equal(!error,r.valid,r.id+': '+error?.message);}
const copy=registeredSourceCatalogV1();copy.sources.length=0;assert.equal(registeredSourceCatalogV1().sources.length,2);
for(const scope of ['autonomous_excerpt','production_chapter']){const checks=requiredAcceptanceChecks(scope);assert.equal(checks.filter(c=>c.implemented).length,8);assert.equal(checks.filter(c=>!c.implemented).length,29);}
assert.equal(acceptanceCheckMatrix('pt-acceptance-required-v3').checks.filter(c=>c.implemented).length,5);
assert.equal(data.records.find(r=>r.id==='missing-mapping-remains-blocked').report.checks.find(c=>c.check_id==='complete-chapter').outcome,'missing');
console.log(`Registered source scopes: ${data.records.length} Rust-produced scope/forgery/archive reports agree; selected scopes retain 29 missing verifiers`);
