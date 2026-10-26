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
