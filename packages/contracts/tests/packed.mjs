import assert from 'node:assert/strict';
import {mkdtemp,mkdir,rm,readFile,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
const temp=await mkdtemp(join(tmpdir(),'pt-contracts-packed-'));
function run(cmd,args,cwd){const r=spawnSync(cmd,args,{cwd,encoding:'utf8'});assert.equal(r.status,0,r.stdout+r.stderr);return r.stdout;}
try{
 const packed=JSON.parse(run('npm',['pack','--workspace','@paper-theater/contracts','--pack-destination',temp,'--cache',join(temp,'cache'),'--json'],process.cwd()));
 const modules=join(temp,'node_modules','@paper-theater');await mkdir(modules,{recursive:true});
 run('tar',['-xzf',join(temp,packed[0].filename),'-C',modules],temp);
 run('mv',[join(modules,'package'),join(modules,'contracts')],temp);
 run(process.execPath,['--input-type=module','--eval',`import assert from 'node:assert/strict';import * as c from '@paper-theater/contracts';assert.equal(c.PROVIDER_METADATA_VERSION,2);assert.equal(c.SERVICE_REVISION,'pt-services-v1.8.0');assert.equal(typeof c.validateProviderResultV2,'function');assert.equal(typeof c.validatePlayback,'function');assert.equal(typeof c.validateDeliveryAudioObservationV1,'function');assert.equal(typeof c.validateSourceSliceRelationshipV1,'function');assert.equal(c.PROVIDER_SERVICE_VERSION,2);assert.equal(typeof c.providerServicesV2,'function');assert.equal(typeof c.validateProviderRecoveryResultV2,'function');assert.equal(c.ACCEPTANCE_REPORT_VERSION,4);assert.equal(typeof c.validateAcceptanceVerificationReport,'function');assert.equal(typeof c.validateTrustedPlayerLoadV1,'function');`],temp);
 await writeFile(join(temp,'consumer.mts'),await readFile('packages/contracts/tests/service-api.ts'));
 run(process.execPath,[join(process.cwd(),'node_modules/typescript/bin/tsc'),'--strict','--skipLibCheck','--noEmit','--module','nodenext','--target','ES2022',join(temp,'consumer.mts')],temp);
 console.log('contracts: isolated packed native Node exports and strict consumer pass');
}finally{await rm(temp,{recursive:true,force:true});}
