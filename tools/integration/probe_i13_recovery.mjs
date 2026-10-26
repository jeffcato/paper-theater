// Public-API edge probes, separate from the existing successful P16 browser matrix.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname,join} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const args=new Map();for(let i=2;i<process.argv.length;i+=2)args.set(process.argv[i],process.argv[i+1]);
const root=resolve(args.get('--workspace')||join(dirname(fileURLToPath(import.meta.url)),'../..'));
const p=await import(pathToFileURL(join(root,'packages/player/dist/src/index.js')));
const bytes=await readFile(join(root,'fixtures/player/two-scene-local/manifest.diagnostic.json'));
const acceptance=await readFile(join(root,'fixtures/player/two-scene-local/acceptance.json'));
const handle=await p.loadDiagnostic(bytes,await p.sha256Hex(bytes),async()=>acceptance,{player_sha256:'0'.repeat(64),checker_sha256:'0'.repeat(64),recipe_sha256:'0'.repeat(64),allow_diagnostic:false});
const timeline=JSON.parse(await readFile(join(root,'fixtures/player/two-scene-local/timeline.json'),'utf8'));
const clock=()=>({currentTime:0,paused:true,seeking:false,ended:false,async play(){this.paused=false;},pause(){this.paused=true;}});
const sink=canvas=>({canvas,render(){},dispose(){}});
const errors=[];const onUnhandled=e=>errors.push(String(e));process.on('unhandledRejection',onUnhandled);
let restore;
try{
 const media=clock(),canvas=new EventTarget(),mount=p.mountPlayer({}, {handle,timeline},{clock:media,sink:sink(canvas)});
 try{canvas.dispatchEvent(new Event('webglcontextlost',{cancelable:true}));canvas.dispatchEvent(new Event('webglcontextrestored'));await new Promise(r=>setTimeout(r,25));restore={status:mount.graphicsStatus(),unhandled_rejections:[...errors]};}finally{mount.dispose();}
}finally{process.removeListener('unhandledRejection',onUnhandled);}
let release;const media=clock();media.play=function(){return new Promise(r=>{release=()=>{this.paused=false;r();};});};
const canvas=new EventTarget(),mount=p.mountPlayer({}, {handle,timeline},{clock:media,sink:sink(canvas)});let race;
try{const play=mount.play();await Promise.resolve();canvas.dispatchEvent(new Event('webglcontextlost',{cancelable:true}));release();await play;race={paused:media.paused,graphics:mount.graphicsStatus().state,reason:mount.reason};}finally{mount.dispose();}
const findings=[];
if(restore.unhandled_rejections.length)findings.push({id:'p16-unhandled-restore-rejection',severity:'medium'});
if(race.graphics==='lost'&&!race.paused)findings.push({id:'p16-pending-play-resumes-during-loss',severity:'high'});
const report={tool:'i13-recovery-edge-probes',runtime:process.version,fixture_manifest_sha256:await p.sha256Hex(bytes),mount_source_sha256:createHash('sha256').update(await readFile(join(root,'packages/player/src/mount.ts'))).digest('hex'),probe_sha256:createHash('sha256').update(await readFile(fileURLToPath(import.meta.url))).digest('hex'),restore_without_factory:restore,loss_during_pending_play:race,findings,provider_calls:0,production_approved:false};
if(args.has('--output')){const output=resolve(args.get('--output'));await mkdir(dirname(output),{recursive:true});await writeFile(output,JSON.stringify(report,null,2)+'\n');}
console.log(JSON.stringify({findings,provider_calls:0}));if(findings.length)process.exitCode=1;
