import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../../../',import.meta.url));
const build=spawnSync('npm',['exec','--','tsc','-p','packages/contracts/tsconfig.json'],{cwd:root,encoding:'utf8'});
assert.equal(build.status,0,build.stdout+build.stderr);
// Exercise the public package export, not a path that bypasses package.json.
const publicImport=spawnSync(process.execPath,['--input-type=module','--eval',
  "const c=await import('@paper-theater/contracts');if(typeof c.validatePlayback!=='function'||!c.CONTRACT_REVISION)process.exit(1)"],{cwd:root,encoding:'utf8'});
assert.equal(publicImport.status,0,publicImport.stdout+publicImport.stderr);
const declarations=spawnSync('npm',['exec','--','tsc','-p','packages/contracts/tests/tsconfig.json'],{cwd:root,encoding:'utf8'});
assert.equal(declarations.status,0,declarations.stdout+declarations.stderr);
const {validatePlayback,productionBlockers}=await import('@paper-theater/contracts');
const result=spawnSync('cargo',['run','--offline','--quiet','-p','pt-contracts','--example','contract-fixtures'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(result.status,0,result.stderr);
const records=JSON.parse(result.stdout);
let count=0;
for(const record of records){
 if(record.valid){await validatePlayback(record.document);assert(productionBlockers(record.document).length);}
 else{let failure;try{await validatePlayback(record.document);}catch(e){failure=e;}assert(failure,record.id+' was accepted');assert.equal(failure.code,record.code,record.id+': '+failure.message);assert(failure.field,record.id+' missing field');}
 count++;
}
console.log(`contracts: ${count} Rust-produced records agree in TypeScript; diagnostic release remains blocked`);
const provider=spawnSync(process.execPath,['packages/contracts/tests/provider-metadata.mjs'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(provider.status,0,provider.stdout+provider.stderr);
process.stdout.write(provider.stdout);
const packed=spawnSync(process.execPath,['packages/contracts/tests/packed.mjs'],{cwd:root,encoding:'utf8'});
assert.equal(packed.status,0,packed.stdout+packed.stderr);process.stdout.write(packed.stdout);
const acceptance=spawnSync(process.execPath,['packages/contracts/tests/acceptance.mjs'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(acceptance.status,0,acceptance.stdout+acceptance.stderr);process.stdout.write(acceptance.stdout);

const providerService=spawnSync(process.execPath,['packages/contracts/tests/provider-service.mjs'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(providerService.status,0,providerService.stdout+providerService.stderr);process.stdout.write(providerService.stdout);

const sourceSlice=spawnSync(process.execPath,['packages/contracts/tests/source-slice.mjs'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});assert.equal(sourceSlice.status,0,sourceSlice.stdout+sourceSlice.stderr);process.stdout.write(sourceSlice.stdout);

const audioFormat=spawnSync(process.execPath,['packages/contracts/tests/audio-format.mjs'],{cwd:root,encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(audioFormat.status,0,audioFormat.stdout+audioFormat.stderr);process.stdout.write(audioFormat.stdout);
const pilotReview=spawnSync(process.execPath,['packages/contracts/tests/pilot-review.mjs'],{cwd:root,encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(pilotReview.status,0,pilotReview.stdout+pilotReview.stderr);process.stdout.write(pilotReview.stdout);
