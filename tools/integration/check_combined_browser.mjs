// Reuse the studio's browser test, adding version-bound readiness/capture probes.
import {spawn} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import {readFile,writeFile,mkdir,rm} from 'node:fs/promises';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const options=new Map();for(let i=2;i<process.argv.length;i+=2)options.set(process.argv[i],process.argv[i+1]);
const root=resolve(options.get('--workspace')||join(dirname(fileURLToPath(import.meta.url)),'../..'));
const fixture=options.get('--fixture')||'fixtures/generation/two-scene-readiness-v2';
const output=resolve(options.get('--output')||join(root,'docs/implementation/integration/combined-browser-v2'));
const sourcePath=join(root,'apps/studio/scripts/browser-check.mjs');
const original=await readFile(sourcePath,'utf8');
const expectedManifest=JSON.parse(await readFile(join(root,fixture,'manifest.json'),'utf8'));
const expectedTimeline=JSON.parse(await readFile(join(root,fixture,'timeline.json'),'utf8'));
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
await mkdir(output,{recursive:true});
const gate=` const expectedManifestBytes=await readFile(join(root,process.env.PT_COMBINED_FIXTURE,'manifest.json'));
 const manifestResponse=await fetch(url+'/diagnostic/manifest.json');
 assert.equal(manifestResponse.status,200);
 const cryptoModule=await import('node:crypto');
 assert.equal(cryptoModule.createHash('sha256').update(new Uint8Array(await manifestResponse.arrayBuffer())).digest('hex'),cryptoModule.createHash('sha256').update(expectedManifestBytes).digest('hex'),'configured fixture mismatch');
`;
async function probe(expected){
 const check=(ok,message)=>{if(!ok)throw new Error(message);};
 const urls=performance.getEntriesByType('resource').map(r=>r.name).filter(u=>new URL(u).pathname.startsWith('/assets/')&&new URL(u).pathname.endsWith('.js'));
 let api;
 for(const url of urls){const mod=await import(url);if(typeof mod.mountPlayer==='function')api=mod;}
 check(api,'loaded studio bundle did not expose the shared player');
 const run=new URLSearchParams(location.search).get('run');
 const ref=await(await fetch('/api/v1/runs/'+run+'/diagnostic')).json();check(ref,'stored diagnostic package unavailable');
 const read=async(path,max)=>{const r=await fetch(new URL(path,new URL(ref.manifest_path,location.href)));check(r.ok,'artifact fetch');const b=new Uint8Array(await r.arrayBuffer());check(b.length<=max,'artifact bounds');return b;};
 const bytes=new Uint8Array(await(await fetch(ref.manifest_path)).arrayBuffer());const manifest=JSON.parse(new TextDecoder().decode(bytes));
 check(manifest.entries.audio.sha256===expected.audio,'stored master differs from selected fixture');
 const policy={player_sha256:'',checker_sha256:'',recipe_sha256:'',allow_diagnostic:true};
 let rejected=false;try{await api.loadRelease(bytes,ref.manifest_sha256,read,policy);}catch{rejected=true;}check(rejected,'production loader accepted diagnostic media');
 const handle=await api.loadDiagnostic(bytes,ref.manifest_sha256,read,policy);
 const timeline=JSON.parse(new TextDecoder().decode(await read(manifest.entries.timeline.path,1024*1024)));
 check(JSON.stringify(timeline.transitions)===JSON.stringify(expected.transitions),'stored transition differs from selected fixture');
 const sha=async b=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new Uint8Array(b)))].map(x=>x.toString(16).padStart(2,'0')).join('');
 const cards=JSON.parse(new TextDecoder().decode(await read(manifest.entries.actors.path,1024*1024)));const imageUrls=[];const figures=[];
 for(const card of cards){const image=await read(card.image.path,32*1024*1024);check(await sha(image)===card.image.sha256,'figure hash');const url=URL.createObjectURL(new Blob([image],{type:card.image.media_type}));imageUrls.push(url);figures.push({asset_id:card.id,url,pixel_size:card.pixel_size,pivot_top_left:card.pivot_top_left,stage_size:card.stage_size,allow_mirror:card.allow_mirror});}
 const sink=await api.createThreeSink(960,540,{figures,calibration:false});const mount=api.mountPlayer(document.createElement('div'),{handle,timeline},{sink});
 const tr=timeline.transitions[0],cue=timeline.cues.find(c=>c.id==='nod-em');
 const samples=[tr.swap_sample,tr.fade_in_start-1,tr.fade_in_start,tr.fade_in_start+1,tr.fade_in_end-1,tr.fade_in_end,tr.fade_in_end+1,...[.25,.5,.75].map(p=>cue.start_sample+Math.floor((cue.end_sample-cue.start_sample)*p)),timeline.sample_count];
 const frames=[];
 try{
  for(const sample of samples){mount.seek(sample);const state=mount.tick();const png=sink.canvas.toDataURL('image/png');const frame=await mount.captureFrame(960,540);check(frame.accepted===false&&frame.sample===sample,'capture identity/diagnostic stamp');const actor=state.actors.find(a=>a.instance_id==='em-2'&&a.present);if(sample===tr.fade_in_start)check(!!actor===expected.ready,'incoming-picture readiness disagrees with evidence bound');if(sample>=cue.start_sample&&sample<cue.end_sample)check(actor?.gesture==='nod'&&actor.gesture_intensity>0,'nod is absent');if(sample===tr.swap_sample||sample===tr.fade_in_start)check(state.stage_gain===0,'dark hold gain');if(sample===timeline.sample_count)check(state.curtain_open_ratio===0,'terminal curtain');frames.push({sample,phase:frame.phase,scene_id:state.scene_id,stage_gain:state.stage_gain,actor:actor||null,manifest_sha256:frame.manifest_digest,profile_id:frame.profile_id,state_sha256:frame.state_digest,rgba_sha256:await sha(frame.pixels),png});}
 }finally{mount.dispose();sink.dispose();for(const url of imageUrls)URL.revokeObjectURL(url);}
 const nod=frames.filter(f=>f.sample>=cue.start_sample&&f.sample<cue.end_sample);
 const bundles=[];for(const url of urls){const b=new Uint8Array(await(await fetch(url)).arrayBuffer());bundles.push({path:new URL(url).pathname,sha256:await sha(b)});}
 return {manifest_sha256:ref.manifest_sha256,audio_sha256:manifest.entries.audio.sha256,incoming_picture_eligible_at_fade_start:expected.ready,production_rejected:true,nod_pixel_changes:new Set(nod.map(f=>f.rgba_sha256)).size>1,bundles,frames};
}
const group=expectedManifest.reveals.find(g=>g.id==='reveal-2');
const expected={audio:expectedManifest.entries.audio.sha256,transitions:expectedTimeline.transitions,ready:group.not_before_sample!==null&&group.not_before_sample<=expectedTimeline.transitions[0].fade_in_start};
const extra=` const browserProbe=await evaluate('('+${JSON.stringify(probe.toString())}+')('+${JSON.stringify(JSON.stringify(expected))}+')');
 for(const frame of browserProbe.frames){const data=Buffer.from(frame.png.split(',')[1],'base64');const name='sample-'+frame.sample+'.png';await writeFile(join(process.env.PT_COMBINED_OUTPUT,name),data);frame.png_sha256=(await import('node:crypto')).createHash('sha256').update(data).digest('hex');frame.png_path=name;delete frame.png;}
 browserProbe.browser=await call('Browser.getVersion');
 await writeFile(join(process.env.PT_COMBINED_OUTPUT,'browser-probe.json'),JSON.stringify(browserProbe,null,2)+String.fromCharCode(10));
`;
if(!original.includes(' const chrome=spawn')||!original.includes(" const metrics=await call('Page.getLayoutMetrics');"))throw new Error('studio browser harness changed; review instrumentation');
let script=original.replace(' const chrome=spawn',gate+' const chrome=spawn').replace(" const metrics=await call('Page.getLayoutMetrics');",extra+" const metrics=await call('Page.getLayoutMetrics');");
if(original.split("join(root,'fixtures/shared/two-scene-v1')").length!==3)throw new Error('fixture argument sites changed');
script=script.replaceAll("join(root,'fixtures/shared/two-scene-v1')","join(root,process.env.PT_COMBINED_FIXTURE)");
script=script.replace('currentTime>0.1','currentTime>5.1');
script=script.replace(" await evaluate(`document.querySelector('[data-testid=\"player-pause\"]').click()`);"," assert(await evaluate(`Number(document.querySelector('[data-testid=\"player-sample\"]')?.textContent)>240000`),'shared player stopped before the first full picture');\n await evaluate(`document.querySelector('[data-testid=\"player-pause\"]').click()`);");
script=script.replace("join(root,'docs/implementation/runtime/studio-browser.png')","join(process.env.PT_COMBINED_OUTPUT,'studio.png')");
const generated=join(dirname(sourcePath),'.combined-'+randomUUID()+'.mjs');
try{
 await writeFile(generated,script);
 const child=spawn(process.execPath,[generated],{cwd:root,env:{...process.env,PT_COMBINED_FIXTURE:fixture,PT_COMBINED_OUTPUT:output,PT_SERVER_BIN:options.get('--binary')||process.env.PT_SERVER_BIN||join(root,'target/debug/pt-server')},stdio:['ignore','pipe','pipe']});
 let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
 const code=await new Promise((r,j)=>{child.on('error',j);child.on('exit',r);});
 await writeFile(join(output,'worker-check.log'),stdout+stderr);if(code!==0)throw new Error(stdout+stderr);
 const result=JSON.parse(await readFile(join(output,'browser-probe.json'),'utf8'));
 result.fixture_path=fixture;result.input_manifest_sha256=hash(await readFile(join(root,fixture,'manifest.json')));result.worker_harness_sha256=hash(original);result.integration_harness_sha256=hash(await readFile(fileURLToPath(import.meta.url)));result.worker_flow=JSON.parse(stdout.trim().split('\n').at(-1));result.provider_calls=0;result.production_accepted=false;
 await writeFile(join(output,'receipt.json'),JSON.stringify(result,null,2)+'\n');
 console.log(JSON.stringify({fixture,frames:result.frames.length,readiness:result.incoming_picture_eligible_at_fade_start,nod_pixel_changes:result.nod_pixel_changes,production_rejected:true,worker_flow:result.worker_flow}));
}finally{await rm(generated,{force:true});}
