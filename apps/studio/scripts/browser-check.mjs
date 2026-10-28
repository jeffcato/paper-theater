import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import assert from 'node:assert/strict';
const root=resolve(import.meta.dirname,'../../..');
const temp=await mkdtemp(join(tmpdir(),'pt-studio-browser-'));
const children=[];let ws;let url;
const eventType=e=>{try{return JSON.parse(e.summary).type||e.kind;}catch{return e.kind;}};
async function until(fn,message,timeout=15000){const start=Date.now();while(Date.now()-start<timeout){try{const result=await fn();if(result)return result;}catch{}await delay(50);}throw new Error(message);}
try{
 const server=spawn(process.env.PT_SERVER_BIN||join(root,'target/debug/pt-server'),['--data',join(temp,'data'),'--port','0','--studio',join(root,'apps/studio/dist'),'--fixture',(process.env.PT_STUDIO_FIXTURE?join(root,process.env.PT_STUDIO_FIXTURE):join(root,'fixtures/shared/two-scene-v1'))],{cwd:root,stdio:['ignore','pipe','pipe']});children.push(server);server.on('error',()=>{});server.stdout.on('data',b=>{url??=String(b).match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];});
 await until(()=>url,'server did not start');
 const chrome=spawn(process.env.PT_CHROME||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--no-first-run','--no-default-browser-check','--disable-background-networking','--remote-debugging-port=0',`--user-data-dir=${join(temp,'chrome')}`,'about:blank'],{stdio:'ignore'});children.push(chrome);chrome.on('error',()=>{});
 const port=await until(async()=>String(await readFile(join(temp,'chrome','DevToolsActivePort'))).split('\n')[0],'Chrome did not start');
 const page=await (await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`,{method:'PUT'})).json();
 ws=new WebSocket(page.webSocketDebuggerUrl);await new Promise((r,j)=>{ws.onopen=r;ws.onerror=j;});let sequence=0;const pending=new Map();const requests=[];const consoleEntries=[];let scenario="startup";
 ws.onmessage=({data})=>{const m=JSON.parse(data);if(m.id){const p=pending.get(m.id);if(!p)return;pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result);}if(m.method==='Network.requestWillBeSent')requests.push(m.params.request);if(m.method==='Log.entryAdded')consoleEntries.push({...m.params.entry,scenario});if(m.method==='Runtime.consoleAPICalled'&&['error','warning'].includes(m.params.type))consoleEntries.push({scenario,level:m.params.type,text:m.params.args.map(a=>a.value??a.description).join(' ')});};
 const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;const timer=setTimeout(()=>{pending.delete(id);reject(new Error('Chrome timed out: '+method));},30000);pending.set(id,{resolve:value=>{clearTimeout(timer);resolve(value);},reject:error=>{clearTimeout(timer);reject(error);}});ws.send(JSON.stringify({id,method,params}));});
 const evaluate=async(expression)=>{const result=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true,userGesture:true}).catch(error=>{throw new Error(error.message+' '+expression.slice(0,350));});if(result.exceptionDetails)throw new Error((result.exceptionDetails.exception?.description||result.exceptionDetails.text)+' '+expression.slice(0,200));return result.result.value;};
 await call('Network.enable');await call('Page.enable');await call('Log.enable');await call('Runtime.enable');if(process.env.PT_STUDIO_PLAYER_CHECK){const {auditPreview}=await import('./studio-player-check.mjs');const source='('+auditPreview.toString()+')()';await call('Page.addScriptToEvaluateOnNewDocument',{source});await evaluate(source);}await call('Emulation.setDeviceMetricsOverride',{width:1440,height:1100,deviceScaleFactor:1,mobile:false});
 await until(()=>evaluate(`!!document.querySelector('[data-testid="start"]')`),'missing start control');
 let seq=[];
 if(process.env.PT_STUDIO_PREVIEW_ONLY){
  await until(()=>evaluate(`!document.querySelector('[data-testid="start"]').disabled`),'start unavailable');await evaluate(`document.querySelector('[data-testid="start"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'fixture did not start');await evaluate(`document.querySelector('[data-testid="cancel"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'fixture did not cancel');
  await until(()=>evaluate(`!document.querySelector('[data-testid="saved-diagnostic"]').disabled`),'saved package unavailable');await evaluate(`document.querySelector('[data-testid="saved-diagnostic"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='failed'`),'diagnostic run did not finish',40000);
 }else{
 await evaluate(`document.querySelector('[data-testid="start"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'run did not start');
 await evaluate(`document.querySelector('[data-testid="cancel"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'cancel not persisted');
 await call('Page.reload');await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'cancel lost after reload');
 await evaluate(`document.querySelector('[data-testid="replay"]').click()`);const before=requests.length;
 await until(()=>evaluate(`document.querySelector('[data-testid="mode"]')?.textContent==='Recorded replay'`),'replay unavailable');
 await delay(250);assert.equal(requests.slice(before).filter(r=>r.method==='POST').length,0,'replay mutated server');
 await evaluate(`document.querySelector('[data-testid="start"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'second run did not start');
 scenario='intentional-disconnect';await call('Network.emulateNetworkConditions',{offline:true,latency:0,downloadThroughput:-1,uploadThroughput:-1});await delay(400);await call('Network.emulateNetworkConditions',{offline:false,latency:0,downloadThroughput:-1,uploadThroughput:-1});
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='succeeded'`),'run failed to reconnect',20000);scenario='running';
 seq=await evaluate(`Array.from(document.querySelectorAll('[data-event-seq]'),e=>Number(e.dataset.eventSeq))`);assert(seq.length>12);assert.equal(new Set(seq).size,seq.length);assert(seq.every((n,i)=>i===0||n===seq[i-1]+1),'journal lost an event');
 await evaluate(`document.querySelector('[data-testid="start"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'restart run did not start');
 await until(()=>evaluate(`fetch('/api/v1/runs/'+new URLSearchParams(location.search).get('run')).then(r=>r.json()).then(s=>s.stages[0].outcome==='complete')`),'source checkpoint not committed');
 scenario='intentional-server-restart';server.kill('SIGKILL');await new Promise(resolve=>server.once('exit',resolve));
 const replacement=spawn(process.env.PT_SERVER_BIN||join(root,'target/debug/pt-server'),['--data',join(temp,'data'),'--port',new URL(url).port,'--studio',join(root,'apps/studio/dist'),'--fixture',(process.env.PT_STUDIO_FIXTURE?join(root,process.env.PT_STUDIO_FIXTURE):join(root,'fixtures/shared/two-scene-v1'))],{cwd:root,stdio:'ignore'});children.push(replacement);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='succeeded' && document.querySelector('.journal')?.textContent.includes('cache reused')`),'server restart did not reuse committed checkpoint',20000);scenario='running';
 assert(await evaluate(`!!document.querySelector('[data-testid="saved-diagnostic"]')`),'saved package control missing');
 await evaluate(`document.querySelector('[data-testid="saved-diagnostic"]').click()`);
 let cancelledAcousticRun;
 await until(async()=>{cancelledAcousticRun=await evaluate(`new URLSearchParams(location.search).get('run')`);const journal=await(await fetch(url+'/api/v1/runs/'+cancelledAcousticRun+'/journal')).json();return journal.events?.some(e=>eventType(e)==='helper_authorized');},'real acoustic helper was not authorized',40000);
 await evaluate(`document.querySelector('[data-testid="cancel"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'acoustic cancellation did not persist');
 await delay(150);
 const cancelledSnapshot=await(await fetch(url+'/api/v1/runs/'+cancelledAcousticRun)).json();assert.equal(cancelledSnapshot.state,'cancelled');assert(!cancelledSnapshot.stages.find(s=>s.stage==='acoustic').artifacts.some(a=>a.file.path==='acoustic-validation.json'),'cancel accepted late acoustic output');
 assert.equal(await(await fetch(url+'/api/v1/runs/'+cancelledAcousticRun+'/diagnostic')).json(),null,'cancel published a diagnostic package');
 await evaluate(`document.querySelector('[data-testid="saved-diagnostic"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='failed'`),'diagnostic findings were not retained',40000);
 const acousticRun=await evaluate(`new URLSearchParams(location.search).get('run')`);
 const acousticSnapshot=await(await fetch(url+'/api/v1/runs/'+acousticRun)).json();const acousticStage=acousticSnapshot.stages.find(s=>s.stage==='acoustic');assert.equal(acousticStage.outcome,'needs_repair');
 const validationRef=acousticStage.artifacts.find(a=>a.file.path==='acoustic-validation.json');assert(validationRef,'real acoustic validation was not persisted');
 const validation=await(await fetch(url+'/api/v1/artifacts/'+validationRef.id)).json();assert.equal(validation.word_count,176);assert.equal(validation.highlight_group_count,170);assert.equal(validation.provider_calls,0);assert.equal(validation.production_approved,false);assert(validation.missing_evidence.includes('independent-acoustic-evidence'));
 const acousticJournal=await(await fetch(url+'/api/v1/runs/'+acousticRun+'/journal')).json();assert.equal(acousticJournal.events.filter(e=>eventType(e)==='helper_authorized').length,1);assert(acousticJournal.events.every(e=>!eventType(e).startsWith('provider_')));
 const replayRequests=requests.length;await evaluate(`document.querySelector('[data-testid="replay"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="mode"]')?.textContent==='Recorded replay'`),'acoustic replay not restored');await delay(150);
 const replaySnapshot=await(await fetch(url+'/api/v1/runs/'+acousticRun)).json();assert.equal(replaySnapshot.last_event_seq,acousticSnapshot.last_event_seq,'replay mutated persisted acoustic work');assert.equal(requests.slice(replayRequests).filter(r=>r.method==='POST').length,0,'acoustic replay dispatched work');

 assert(await evaluate(`document.body.textContent.includes('Needs repair')`),'missing acceptance not shown');
 const sourceDecision=acousticJournal.events.find(e=>e.kind==='director_decision'&&JSON.parse(e.summary).record.inputs.some(a=>a.file.path==='frozen-source.json'));assert(sourceDecision,'source-linked package decision missing');
 await evaluate(`document.querySelector('[data-event-seq="${sourceDecision.seq}"] button').click()`);
 await until(()=>evaluate(`Array.from(document.querySelectorAll('.artifact-link')).some(e=>e.textContent.includes('frozen-source.json'))`),'recorded decision has no source link');
 await evaluate(`Array.from(document.querySelectorAll('.artifact-link')).find(e=>e.textContent.includes('frozen-source.json')).click()`);
 await until(()=>evaluate(`document.querySelector('.artifact pre')?.textContent.includes('Dorothy lived')`),'source bytes missing from inspector');

 }
 assert(await evaluate(`!!document.querySelector('[data-testid="preview-load"]')`),'shared preview control missing');
 await evaluate(`document.querySelector('[data-testid="preview-load"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="player-status"]')?.textContent.includes('Diagnostic mount')`),'shared diagnostic mount unavailable',30000);
 assert(await evaluate(`!!document.querySelector('.player-surface canvas')`),'shared renderer canvas was not attached');
 await evaluate(`document.querySelector('[data-testid="player-seek"]').focus()`);
 await call('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowRight',code:'ArrowRight'});await call('Input.dispatchKeyEvent',{type:'keyUp',key:'ArrowRight',code:'ArrowRight'});
 await until(()=>evaluate(`Number(document.querySelector('[data-testid="player-sample"]')?.textContent)>0`),'shared player seek did not update sampled state');
 await evaluate(`document.querySelector('[data-testid="player-play"]').click()`);
 await until(()=>evaluate(`!document.querySelector('.preview audio').paused && document.querySelector('.preview audio').currentTime>0.1`),'verified audio did not play');
 await evaluate(`document.querySelector('[data-testid="player-pause"]').click()`);
 assert(await evaluate(`document.querySelector('.inspector').getBoundingClientRect().height<=900`),'inspector must scroll within its column');
 scenario='studio-preview';if(process.env.PT_STUDIO_PLAYER_CHECK)await (await import('./studio-player-check.mjs')).checkStudioPreview({call,evaluate,until,root,url,requests,consoleEntries});
 const metrics=await call('Page.getLayoutMetrics');
 const capture=await call('Page.captureScreenshot'  ,{format:'png',captureBeyondViewport:true,clip:{x:0,y:0,width:1440,height:metrics.cssContentSize.height,scale:1}});await writeFile(join(root,'docs/implementation/runtime/studio-browser.png'),Buffer.from(capture.data,'base64'));
 console.log(JSON.stringify(process.env.PT_STUDIO_PREVIEW_ONLY?{studio_preview_only:true}: {real_acoustic:true,acoustic_cancel:true,acoustic_replay_read_only:true,start:true,cancel:true,reload:true,reconnect:true,replay_zero_posts:true,process_restart_reuse:true,diagnostic_needs_repair:true,recorded_source_link:true,shared_player_mount:true,shared_canvas:true,verified_audio_play:true,events:seq.length}));
}finally{ws?.close();for(const child of children.reverse())child.kill('SIGKILL');await delay(200);await rm(temp,{recursive:true,force:true});}
