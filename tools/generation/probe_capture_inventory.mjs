// Offline interoperability probe of planned requests; produces evaluated states, never images.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'../..');
const args=process.argv.slice(2);
const option=(name)=>{const i=args.indexOf(name);if(i<0 || !args[i+1])throw Error(`Missing ${name}`);return args[i+1];};
const hash=(bytes)=>createHash('sha256').update(bytes).digest('hex');
const stable=(value)=>Array.isArray(value)?value.map(stable):value && typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(k=>[k,stable(value[k])])):value;
const canonical=(value)=>JSON.stringify(stable(value))+'\n';
const inventoryBytes=readFileSync(option('--inventory'));
const inventory=JSON.parse(inventoryBytes);
const read=(path)=>{
    if(path.startsWith('/') || path.split('/').includes('..'))throw Error('Unsafe input path');
    return readFileSync(resolve(root,path));
};
for(const record of [...inventory.bindings.player_files,...inventory.bindings.player_sources]){
    const bytes=read(record.path);
    if(bytes.length!==record.bytes || hash(bytes)!==record.sha256)throw Error(`Changed player input: ${record.path}`);
}
if(hash(canonical(inventory.bindings.player_files))!==inventory.bindings.player_sha256)throw Error('Player binding mismatch');
if(hash(read(inventory.bindings.checker_path))!==inventory.bindings.checker_sha256)throw Error('Changed inventory checker');
const base=inventory.fixture_path;
if(hash(read(base+'/manifest.json'))!==inventory.bindings.manifest_sha256 || hash(read(base+'/profile.json'))!==inventory.bindings.profile_sha256)throw Error('Changed fixture binding');
if(hash(read(base+'/timeline.json'))!==inventory.bindings.timeline_sha256)throw Error('Changed authored timeline');
const doc=JSON.parse(read(base+'/fixture.json'));
const {describeCapture}=await import(pathToFileURL(resolve(root,'packages/player/dist/src/capture.js')));
const {evaluate}=await import(pathToFileURL(resolve(root,'packages/player/dist/src/evaluate.js')));
const {toCueView}=await import(pathToFileURL(resolve(root,'packages/player/dist/src/shared.js')));
const timeline={...doc.timeline,cues:doc.timeline.cues.map(cue=>({...cue,...toCueView(cue)}))};
const results=[];
for(const item of inventory.requests){
    const r=item.request;
    if(r.manifest_digest!==inventory.bindings.manifest_sha256 || r.player_sha256!==inventory.bindings.player_sha256
       || r.checker_sha256!==inventory.bindings.checker_sha256 || r.profile_id!==inventory.bindings.profile_id
       || r.sample!==item.sample || !Number.isSafeInteger(r.sample) || r.sample<0 || r.sample>inventory.sample_count)throw Error('Invalid capture request');
    results.push({request_id:item.id,...await describeCapture(timeline,doc.manifest.reveals,r)});
}
const action_observations=inventory.actions.map(action=>({id:action.id,target_id:action.target_id,
    samples:action.interior_samples.map(sample=>{
        const state=evaluate(timeline,sample,doc.manifest.reveals);
        const actor=state.actors.find(a=>a.instance_id===action.target_id || a.asset_id===action.target_id);
        return {sample,target_present:actor?.present??false,gesture:actor?.gesture??null,
                gesture_intensity:actor?.gesture_intensity??null,position:actor?.position??null,
                rotation_xyz:actor?.rotation_xyz??null};
    }),rendered_motion:'unverified; actual paired frames/clip are required'}));
const findings=[];
for(const action of inventory.actions){
    const observed=action_observations.find(a=>a.id===action.id);
    if(action.action==='gesture' && observed.samples.some(a=>!a.target_present || a.gesture!==action.params.gesture))
        findings.push({id:`action:${action.id}:gesture-state`,outcome:'fail',
            message:'The authored target has no matching active gesture state at interior samples; player owner must resolve before rendered action review.'});
}
const output={schema_version:1,kind:'capture-state-probe',rendered:false,production_approved:false,provider_calls:0,
    inventory_sha256:hash(inventoryBytes),probe_sha256:hash(read('tools/generation/probe_capture_inventory.mjs')),
    player_sha256:inventory.bindings.player_sha256,results,action_observations,findings,
    scope:'Actual compiled player API execution; no browser/pixels or creative/rendered acceptance.'};
const destination=resolve(option('--output'));mkdirSync(dirname(destination),{recursive:true});
writeFileSync(destination,JSON.stringify(output,null,2)+'\n');
process.stdout.write(`Validated ${results.length} state requests; rendered: false; provider calls: 0; action findings: ${findings.length}\n`);
