import React,{useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import type {RunSnapshot,RunEvent,RunRequest,ArtifactRef,ReleaseRef} from '@paper-theater/contracts';
import {EventCursor,eventRecord,terminal} from './state';
import './style.css';
async function api<T>(path:string,options?:RequestInit):Promise<T>{const r=await fetch(`/api/v1/${path}`,options);if(!r.ok){const e=await r.json().catch(()=>({code:`http_${r.status}`}));throw new Error(e.code);}return r.json();}
async function hash(text:string):Promise<string>{return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text))),n=>n.toString(16).padStart(2,'0')).join('');}
const initial=new URLSearchParams(location.search);
type Book={id:string;source_id:string;title:string};
type Books={books:Book[];diagnostic:ReleaseRef|null};
function Studio(){
 const [books,setBooks]=useState<Books>({books:[],diagnostic:null});const [book,setBook]=useState('pg-55');
 const [runs,setRuns]=useState<RunSnapshot[]>([]);const [run,setRun]=useState(initial.get('run')||'');const [replay,setReplay]=useState(initial.get('replay')==='1');
 const [snapshot,setSnapshot]=useState<RunSnapshot>();const [events,setEvents]=useState<RunEvent[]>([]);const [selected,setSelected]=useState(Number(initial.get('event'))||0);
 const [artifact,setArtifact]=useState<ArtifactRef>();const [error,setError]=useState('');const [connection,setConnection]=useState('');const [reload,setReload]=useState(0);const [busy,setBusy]=useState(false);
 const refreshRuns=()=>api<{runs:RunSnapshot[]}>('runs').then(v=>setRuns(v.runs));
 useEffect(()=>{api<Books>('books').then(setBooks).catch(e=>setError(String(e)));refreshRuns().catch(e=>setError(String(e)));},[]);
 function selectRun(id:string,recorded:boolean){setRun(id);setReplay(recorded);setSelected(0);setArtifact(undefined);setError('');history.replaceState(null,'',`?run=${encodeURIComponent(id)}${recorded?'&replay=1':''}`);}
 useEffect(()=>{
  if(!run)return;let stopped=false;let stream:EventSource|undefined;let timer:ReturnType<typeof setTimeout>|undefined;const controller=new AbortController();
  setSnapshot(undefined);setEvents([]);setConnection(replay?'Recorded replay':'Loading snapshot');
  const refresh=()=>api<RunSnapshot>(`runs/${run}`,{signal:controller.signal}).then(s=>{if(!stopped)setSnapshot(s);});
  (async()=>{
   const s=await api<RunSnapshot>(`runs/${run}`,{signal:controller.signal});if(stopped)return;setSnapshot(s);
   const history:RunEvent[]=[];let after=0;
   while(after<s.last_event_seq){const page=await api<{events:RunEvent[]}>(`runs/${run}/journal?after=${after}&limit=500`,{signal:controller.signal});if(!page.events.length)throw new Error('snapshot_required');for(const e of page.events){if(e.seq<=s.last_event_seq)history.push(e);}after=page.events.at(-1)!.seq;}
   if(stopped)return;setEvents(history);
   if(replay||terminal(s.state)){setConnection(replay?'Recorded replay':'Saved journal');return;}
   const cursor=new EventCursor(run,s.last_event_seq);stream=new EventSource(`/api/v1/runs/${run}/events?after=${s.last_event_seq}`);
   stream.onopen=()=>{if(!stopped)setConnection('Live journal');};stream.onerror=()=>{if(!stopped)setConnection('Reconnecting');};
   stream.addEventListener('snapshot_required',()=>{stream?.close();if(!stopped)setReload(n=>n+1);});
   stream.onmessage=({data})=>{try{const event=JSON.parse(data) as RunEvent;if(!cursor.accept(event))return;setEvents(existing=>[...existing,event]);if(!timer)timer=setTimeout(()=>{timer=undefined;refresh().catch(e=>setError(String(e)));},100);if(event.kind==='run_terminal'){stream?.close();setConnection('Saved journal');refresh().then(refreshRuns).catch(e=>setError(String(e)));}}catch{stream?.close();if(!stopped)setReload(n=>n+1);}};
  })().catch(e=>{if(!stopped)setError(String(e));});
  return()=>{stopped=true;controller.abort();stream?.close();if(timer)clearTimeout(timer);};
 },[run,replay,reload]);
 async function start(){setBusy(true);setError('');try{
  const chosen=books.books.find(b=>b.id===book);if(!chosen)throw new Error('source_unavailable');
  const request:RunRequest={schema_version:1,source_id:chosen.source_id,book_id:book,mode:'fixture',pins:{recipe_sha256:await hash('fixture-v1'),profile_sha256:await hash('diagnostic-v1'),profile_id:'diagnostic-v1'},limits:{money_microusd:0,requests:0,tokens:0,repairs:0,wall_time_ms:60000}};
  const body=JSON.stringify(request);let pending:{body:string;key:string}|undefined;try{pending=JSON.parse(sessionStorage.getItem('pt-pending-start')||'null')||undefined;}catch{}
  if(pending?.body!==body)pending={body,key:crypto.randomUUID()};sessionStorage.setItem('pt-pending-start',JSON.stringify(pending));
  const s=await api<RunSnapshot>('runs',{method:'POST',headers:{'content-type':'application/json','idempotency-key':pending!.key},body});sessionStorage.removeItem('pt-pending-start');selectRun(s.run_id,false);await refreshRuns();
 }catch(e){setError(String(e));}finally{setBusy(false);}}
 async function cancel(){if(!snapshot)return;try{setSnapshot(await api<RunSnapshot>(`runs/${run}/cancel`,{method:'POST'}));await refreshRuns();}catch(e){setError(String(e));}}
 const artifacts=snapshot?.stages.flatMap(s=>s.artifacts)||[];const chosenEvent=events.find(e=>e.seq===selected);const record=chosenEvent&&eventRecord(chosenEvent);
 function navigateEvent(event:RunEvent){setSelected(event.seq);history.replaceState(null,'',`?run=${run}&replay=${replay?'1':'0'}&event=${event.seq}`);}
 return <main><header><div><p className="eyebrow">Paper Theater / Production studio</p><h1>The work, as it happened.</h1><p>Source, recorded decisions, artifacts, and rehearsal.</p></div><span className="badge">Fixture · zero provider requests</span></header>
 <section className="toolbar"><label>First chapter <select aria-label="Book" value={book} onChange={e=>setBook(e.target.value)}>{books.books.map(b=><option key={b.id} value={b.id}>{b.title}</option>)}</select></label><button data-testid="start" disabled={busy||!books.books.length} onClick={start}>{busy?'Starting…':'Start fixture run'}</button><p className="muted">Synthetic structural work. Production acceptance remains pending.</p></section>
 {error&&<p role="alert" className="error">{error}</p>}
 <div className="workspace"><aside><h2>Runs</h2>{!runs.length&&<p className="muted">No recorded runs yet.</p>}<ul className="runs">{runs.map(r=><li key={r.run_id}><button aria-current={r.run_id===run} onClick={()=>selectRun(r.run_id,terminal(r.state))}><strong>{r.run_id.slice(0,10)}</strong><span>{r.state} · synthetic</span></button></li>)}</ul></aside>
 <section className="journal"><div className="section-head"><h2>Chronological journal</h2>{snapshot&&<span data-testid="run-state" className="badge">{snapshot.state}</span>}</div>
 {snapshot?<><div className="run-controls"><span data-testid="mode">{replay?'Recorded replay':connection}</span><button data-testid="replay" onClick={()=>selectRun(run,true)}>Recorded replay</button>{replay&&<button onClick={()=>selectRun(run,false)}>Live journal</button>}<button data-testid="cancel" disabled={terminal(snapshot.state)||replay} onClick={cancel}>Cancel run</button></div><p className="muted">{snapshot.stages.filter(s=>s.outcome==='complete').length} of {snapshot.stages.length} known stages complete. {artifacts.length} persisted artifacts.</p>
 <ol className="events">{events.map(e=>{const d=eventRecord(e);return <li key={`${e.run_id}:${e.seq}`} data-event-seq={e.seq}><button className={selected===e.seq?'selected':''} onClick={()=>navigateEvent(e)}><span className="seq">{e.seq}</span><div><strong>{d.type.replaceAll('_',' ')}</strong><span>{typeof d.record.stage==='string'?d.record.stage:typeof d.record.summary==='string'?d.record.summary:''}</span></div><time>{new Date(Number(e.timestamp)).toLocaleTimeString()}</time></button></li>;})}</ol></>:<p className="empty">Start a fixture or open a saved run to inspect its journal.</p>}</section>
 <aside className="inspector"><h2>Inspector</h2>{chosenEvent?<><p>Saved event {chosenEvent.seq}</p><pre>{JSON.stringify(record?.record,null,2)}</pre>{chosenEvent.artifact_ids.map(id=><button className="artifact-link" key={id} onClick={()=>setArtifact(artifacts.find(a=>a.id===id))}>Inspect artifact {id.slice(0,8)}</button>)}</>:<p className="muted">Choose a recorded event. Missing decisions remain unavailable.</p>}
 {artifacts.length>0&&<><h3>Persisted artifacts</h3><ul className="artifact-list">{artifacts.map(a=><li key={a.id}><button onClick={()=>setArtifact(a)}>{a.file.path}<small>{a.provenance}</small></button></li>)}</ul></>}{artifact&&<ArtifactInspector artifact={artifact}/>}</aside></div>
 <Preview reference={books.diagnostic}/><footer>Recorded replay reads saved journal and artifact bytes. It makes no provider requests and cannot publish.</footer></main>;
}
function ArtifactInspector({artifact}:{artifact:ArtifactRef}){
 const [text,setText]=useState('');useEffect(()=>{const abort=new AbortController();setText('');if(['application/json','text/plain'].includes(artifact.file.media_type)&&artifact.file.bytes<1024*1024)fetch(`/api/v1/artifacts/${artifact.id}`,{signal:abort.signal}).then(r=>r.text()).then(setText).catch(()=>{});return()=>abort.abort();},[artifact.id]);
 const url=`/api/v1/artifacts/${artifact.id}`;return <section className="artifact"><h3>{artifact.file.path}</h3><p>{artifact.provenance} · {artifact.file.bytes.toLocaleString()} bytes</p><code>{artifact.file.sha256}</code>{artifact.file.media_type.startsWith('image/')&&<img alt={artifact.file.path} src={url}/ >}{artifact.file.media_type.startsWith('audio/')&&<audio controls src={url}/ >}{text&&<pre>{text}</pre>}<a href={url} target="_blank" rel="noreferrer">Open exact stored bytes</a></section>;
}
function Preview({reference}:{reference:ReleaseRef|null}){
 const container=useRef<HTMLDivElement>(null);const [status,setStatus]=useState('Shared player preview is unavailable until its diagnostic boundary is ready.');const instance=useRef<{dispose:()=>void}|null>(null);
 useEffect(()=>()=>instance.current?.dispose(),[]);
 async function load(){try{setStatus('Loading saved diagnostic media…');const player=await import('@paper-theater/player');
  const api=player as unknown as {loadDiagnostic:(...args:unknown[])=>Promise<{missing_acceptance_ids:readonly string[]}>;mount?:(container:HTMLElement,handle:unknown)=>{dispose:()=>void}};
  if(!reference||!api.mount||!container.current){setStatus('Diagnostic player mount pending. Saved fixture remains ineligible for audience release.');return;}
  const bounded=async(path:string,max:number)=>{const response=await fetch(path);if(!response.ok)throw new Error('fixture_unavailable');const declared=Number(response.headers.get('content-length'));if(declared>max)throw new Error('fixture_size');const reader=response.body!.getReader();const pieces:Uint8Array[]=[];let size=0;while(true){const {value,done}=await reader.read();if(done)break;size+=value.length;if(size>max){await reader.cancel();throw new Error('fixture_size');}pieces.push(value);}const result=new Uint8Array(size);let offset=0;for(const p of pieces){result.set(p,offset);offset+=p.length;}return result;};
  const bytes=await bounded(reference.manifest_path,1024*1024);const unavailable='0'.repeat(64);const handle=await api.loadDiagnostic(bytes,reference.manifest_sha256,(path:string,max:number)=>bounded(`/diagnostic/${path}`,max),{player_sha256:unavailable,checker_sha256:unavailable,recipe_sha256:unavailable,allow_diagnostic:true});
  instance.current?.dispose();instance.current=api.mount(container.current,handle);setStatus(`Diagnostic · missing acceptance: ${handle.missing_acceptance_ids.join(', ')||'production verification pending'}`);
 }catch(e){setStatus(`Diagnostic preview unavailable: ${String(e)}`);}}
 return <section className="preview"><div className="section-head"><h2>Shared player rehearsal</h2><button disabled={!reference} onClick={load}>Open diagnostic fixture</button></div><p>{status}</p><div ref={container}/></section>;
}
createRoot(document.getElementById('root')!).render(<Studio/>);
