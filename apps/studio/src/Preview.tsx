import React,{useEffect,useRef,useState} from 'react';
import type {ReadAlong,RegisteredCard,ReleaseRef,Shell} from '@paper-theater/contracts';
import type {CameraPose,FrameSink,PlayerMount} from '@paper-theater/player';
import '@paper-theater/player/fonts/noto-serif.css';

type Session={audio:HTMLAudioElement;root:HTMLDivElement;abort:AbortController;mount:PlayerMount|null;sink:FrameSink|null;urls:Set<string>;frame:number;layoutFrame:number;observer:ResizeObserver|null;disposed:boolean;fit:(reset?:boolean)=>void};
const initialStatus='Open a saved diagnostic package. Production acceptance remains unresolved.';
export function Preview({reference}:{reference:ReleaseRef|null}){
 const surface=useRef<HTMLDivElement>(null);const media=useRef<HTMLAudioElement>(null);const session=useRef<Session|null>(null);
 const [status,setStatus]=useState(initialStatus);const [loaded,setLoaded]=useState(false);const [sample,setSample]=useState(0);const [total,setTotal]=useState(0);const [phase,setPhase]=useState('');const [reason,setReason]=useState('readiness');const [scale,setScale]=useState(1);const [enabled,setEnabled]=useState(true);
 function dispose(){
  const current=session.current;session.current=null;
  if(current&&!current.disposed){current.disposed=true;current.abort.abort();cancelAnimationFrame(current.frame);cancelAnimationFrame(current.layoutFrame);current.observer?.disconnect();if(current.mount)current.mount.dispose();else current.sink?.dispose?.();for(const url of current.urls)URL.revokeObjectURL(url);current.urls.clear();}
  const audio=current?.audio??media.current;if(audio){audio.pause();audio.removeAttribute('src');audio.load();}
  const root=current?.root??surface.current;if(root){root.replaceChildren();for(const key of Object.keys(root.dataset))delete root.dataset[key];}
 }
 useEffect(()=>{dispose();setLoaded(false);setSample(0);setTotal(0);setPhase('');setReason('readiness');setStatus(initialStatus);return dispose;},[reference?.manifest_path,reference?.manifest_sha256]);
 function close(){dispose();setLoaded(false);setSample(0);setTotal(0);setPhase('');setReason('readiness');setStatus(initialStatus);}
 async function load(){
  if(!reference||!surface.current||!media.current)return;
  dispose();const root=surface.current;const audio=media.current;
  const current:Session={audio,root,abort:new AbortController(),mount:null,sink:null,urls:new Set(),frame:0,layoutFrame:0,observer:null,disposed:false,fit:()=>{}};session.current=current;
  const active=()=>session.current===current&&!current.disposed;
  const fail=(error:unknown)=>{if(active()){dispose();setLoaded(false);setStatus(`Diagnostic preview unavailable: ${String(error)}`);}};
  setLoaded(false);setSample(0);setPhase('');setStatus('Verifying saved diagnostic media…');
  try{
   const player=await import('@paper-theater/player');if(!active())return;
   const files=new Map<string,Uint8Array>();
   const bounded=async(path:string,max:number)=>{
    const response=await fetch(path,{signal:current.abort.signal});if(!response.ok)throw new Error('package_unavailable');if(Number(response.headers.get('content-length'))>max){await response.body?.cancel();throw new Error('package_size');}
    if(!response.body)throw new Error('package_body_missing');const reader=response.body.getReader();const pieces:Uint8Array[]=[];let size=0;
    try{while(true){const {value,done}=await reader.read();if(done)break;size+=value.length;if(size>max)throw new Error('package_size');pieces.push(value);}}catch(error){await reader.cancel().catch(()=>{});throw error;}finally{reader.releaseLock();}
    const bytes=new Uint8Array(size);let offset=0;for(const part of pieces){bytes.set(part,offset);offset+=part.length;}return bytes;
   };
   const read=async(path:string,max:number)=>{player.safePackagePath(path);const bytes=await bounded(new URL(path,new URL(reference.manifest_path,location.href)).pathname,max);files.set(path,bytes);return bytes;};
   const bytes=await bounded(reference.manifest_path,1024*1024);
   const handle=await player.loadDiagnostic(bytes,reference.manifest_sha256,read,{player_sha256:'',checker_sha256:'',recipe_sha256:'',allow_diagnostic:true});if(!active())return;
   const json=<T,>(path:string):T=>{const bytes=files.get(path);if(!bytes)throw new Error(`verified_record_missing: ${path}`);return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));};
   const timeline=json<Parameters<typeof player.mountPlayer>[1]['timeline']>(handle.manifest.entries.timeline.path);
   const track=json<ReadAlong>(handle.manifest.entries.read_along.path);const shell=json<Shell>(handle.manifest.entries.shell.path);
   if(track.schema_version!==1||track.policy!=='point-following-same-boundary-v1'||track.delivery_audio_sha256!==handle.manifest.entries.audio.sha256||track.source_sha256!==handle.manifest.source_sha256||track.presentation_map_sha256!==timeline.presentation_map_sha256||!track.words?.length||!track.groups?.length)throw new Error('verified_read_along_invalid');
   for(const word of track.words){if(!Number.isSafeInteger(word.start_sample)||!Number.isSafeInteger(word.end_sample)||word.start_sample<0||word.end_sample<word.start_sample||word.end_sample>handle.manifest.sample_count)throw new Error('verified_read_along_timing_invalid');}
   player.groupWords(track.words,track.groups);
   if(!timeline.opening.title?.trim()||!timeline.opening.chapter_label?.trim())throw new Error('verified_package_title_missing');
   const audioBytes=files.get(handle.manifest.entries.audio.path);if(!audioBytes)throw new Error('verified_audio_missing');
   const objectUrl=(bytes:Uint8Array,type:string)=>{const url=URL.createObjectURL(new Blob([new Uint8Array(bytes)],{type}));current.urls.add(url);return url;};
   const cards=json<RegisteredCard[]>(handle.manifest.entries.actors.path);
   const figures=cards.map(card=>{const image=files.get(card.image.path);if(!image)throw new Error('verified_figure_missing');return{asset_id:card.id,url:objectUrl(image,card.image.media_type),pixel_size:card.pixel_size,pivot_top_left:card.pivot_top_left,stage_size:card.stage_size,allow_mirror:card.allow_mirror};});
   const sink=await player.createThreeSink(960,540,{figures,calibration:false,title:{heading:timeline.opening.title,sub:timeline.opening.chapter_label}});
   if(!active()){sink.dispose?.();return;}current.sink=sink;
   if(!(sink.canvas instanceof HTMLCanvasElement))throw new Error('shared_renderer_surface_missing');sink.canvas.style.width='100%';sink.canvas.style.height='100%';
   const positioner=document.createElement('div');positioner.className='player-plaque';const plaque=document.createElement('div');plaque.className='player-read-along';positioner.append(plaque);root.replaceChildren(sink.canvas,positioner);
   root.dataset.manifest=handle.manifest_digest;root.dataset.profile=player.PROFILE_ID;root.dataset.accepted=String(handle.accepted);root.dataset.title=timeline.opening.title;root.dataset.chapterLabel=timeline.opening.chapter_label;root.dataset.readAlongSha256=handle.manifest.entries.read_along.sha256;
   audio.src=objectUrl(audioBytes,'audio/wav');audio.load();
   const mount=player.mountPlayer(root,{handle,timeline},{clock:audio,sink,readAlong:{words:track.words,groups:track.groups,container:plaque,scale},events:{onState:state=>{if(active()){setSample(state.sample);setPhase(state.phase);root.dataset.sample=String(state.sample);root.dataset.stageGain=String(state.stage_gain);root.dataset.curtainOpen=String(state.curtain_open_ratio);root.dataset.titleVisible=String(state.title_visible);}},onReason:value=>{if(active())setReason(value);},onSign:(pose,progress)=>{if(active()){root.dataset.sign=pose;root.dataset.signProgress=String(progress);}},onError:fail}});current.mount=mount;
   if(!mount.plaque||!await mount.plaque.fontReady)throw new Error('bundled_read_along_font_unavailable');await mount.plaque.ready;if(!active())return;
   root.dataset.fontReady='true';let growth=0;let pose:CameraPose=player.baseCameraPose();let resetPending=false;
   const place=()=>{const viewport={width:root.clientWidth,height:root.clientHeight};const rect=player.projectPlaqueSafe(pose,viewport);Object.assign(positioner.style,{left:`${rect.left}px`,top:`${rect.top}px`,width:`${rect.width}px`,minHeight:`${rect.height}px`});return rect;};
   const fit=()=>{
    if(!active()||!mount.plaque)return;if(resetPending){growth=0;pose=player.baseCameraPose();sink.setCameraPose?.(pose);resetPending=false;}
    const inset=shell.plaque.inset;const worldHeight=inset.y[1]-inset.y[0];if(!(worldHeight>0))throw new Error('verified_plaque_geometry_invalid');
    for(let pass=0;pass<24;pass++){
     const rect=place();mount.plaque.reflow();mount.plaque.setSample(mount.sample);const chunk=plaque.querySelector('.pl-chunk') as HTMLElement;const needed=Math.max(chunk.getBoundingClientRect().height,parseFloat(getComputedStyle(chunk).lineHeight)*shell.profile.line_count[1])+16;
     const next=player.plaqueGrowthSteps(Math.max(0.01,needed*worldHeight/rect.height),worldHeight);
     if(next<=growth){positioner.style.height=`${Math.max(rect.height,needed)}px`;const box=positioner.getBoundingClientRect();const bounds=root.getBoundingClientRect();if(box.bottom>bounds.bottom+1||box.top<bounds.top-1)throw new Error('read_along_refit_unavailable');root.dataset.cameraPose=JSON.stringify(pose);root.dataset.growthSteps=String(growth);root.dataset.layout='ready';return;}
     growth=next;pose=player.envelopeCameraPose([0,0],0,[0,0],false,player.refitShell(growth));sink.setCameraPose?.(pose);
    }throw new Error('read_along_refit_did_not_settle');
   };
   current.fit=(reset=false)=>{if(!active())return;resetPending||=reset;root.dataset.layout='pending';if(!current.layoutFrame)current.layoutFrame=requestAnimationFrame(()=>{current.layoutFrame=0;try{fit();}catch(error){fail(error);}});};
   // Surface changes restart the measure/grow/refit cycle; chunk changes reuse
   // the current framing until more room is needed. All placement uses CSS px.
   current.observer=new ResizeObserver(entries=>current.fit(entries.some(e=>e.target===root)));current.observer.observe(root);current.observer.observe(plaque.querySelector('.pl-chunk')!);
   fit();mount.setReadAlong(enabled);current.fit();setTotal(handle.manifest.sample_count);setLoaded(true);setStatus(`Diagnostic mount · accepted: false · ${handle.missing_acceptance_ids.length} unresolved acceptance references. Bundled read-along font loaded.`);
   const tick=()=>{if(!active())return;try{mount.tick(performance.now());}catch(error){fail(error);return;}current.frame=requestAnimationFrame(tick);};current.frame=requestAnimationFrame(tick);
  }catch(error){fail(error);}
 }
 return <section className="preview"><div className="section-head"><h2>Shared player rehearsal</h2><div className="run-controls"><button data-testid="preview-load" disabled={!reference} onClick={load}>Open diagnostic fixture</button><button data-testid="preview-close" onClick={close}>Close preview</button></div></div><p data-testid="player-status">{status}</p><div className="player-surface" ref={surface}/><audio ref={media} preload="auto" aria-label="Verified diagnostic audio"/><div className="run-controls"><button data-testid="player-play" disabled={!loaded} onClick={()=>{const current=session.current;void current?.mount?.play().catch(error=>{if(session.current===current)setStatus(`Playback unavailable: ${String(error)}`);});}}>Play</button><button data-testid="player-pause" disabled={!loaded} onClick={()=>session.current?.mount?.pause()}>Pause</button><button data-testid="player-restart" disabled={!loaded} onClick={()=>{const mount=session.current?.mount;mount?.pause(false);mount?.restart();mount?.seek(0);if(enabled){mount?.setReadAlong(false);mount?.setReadAlong(true);}session.current?.fit();}}>Restart</button><label>Sample <input data-testid="player-seek" type="range" min={0} max={Math.max(1,total)} step={1} value={sample} disabled={!loaded} onChange={event=>session.current?.mount?.seek(Number(event.target.value))}/></label><output data-testid="player-sample">{sample}</output><span>{phase.replaceAll('_',' ')} · {reason}</span></div><div className="run-controls read-along-controls"><label><input data-testid="player-read-along" type="checkbox" checked={enabled} disabled={!loaded} onChange={event=>{setEnabled(event.target.checked);session.current?.mount?.setReadAlong(event.target.checked);session.current?.fit();}}/>Read along</label><label>Text size <select data-testid="player-text-size" value={scale} disabled={!loaded} onChange={event=>{const next=Number(event.target.value);setScale(next);session.current?.mount?.plaque?.setScale(next);session.current?.fit(true);}}>{[1,1.5,2].map(value=><option key={value} value={value}>{value*100}%</option>)}</select></label></div><p className="muted">Imported partial media and diagnostic geometry. Automated production acceptance and target-device validation remain pending.</p></section>;
}
