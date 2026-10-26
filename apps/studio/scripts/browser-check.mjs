import {spawn} from 'node:child_process';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import assert from 'node:assert/strict';
const root=resolve(import.meta.dirname,'../../..');
const temp=await mkdtemp(join(tmpdir(),'pt-studio-browser-'));
const children=[];let ws;let url;
async function until(fn,message,timeout=15000){const start=Date.now();while(Date.now()-start<timeout){try{const result=await fn();if(result)return result;}catch{}await delay(50);}throw new Error(message);}
try{
 const server=spawn(process.env.PT_SERVER_BIN||join(root,'target/debug/pt-server'),['--data',join(temp,'data'),'--port','0','--studio',join(root,'apps/studio/dist'),'--fixture',join(root,'fixtures/shared/two-scene-v1')],{cwd:root,stdio:['ignore','pipe','pipe']});children.push(server);server.on('error',()=>{});server.stdout.on('data',b=>{url??=String(b).match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];});
 await until(()=>url,'server did not start');
 const chrome=spawn(process.env.PT_CHROME||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--no-first-run','--no-default-browser-check','--disable-background-networking','--remote-debugging-port=0',`--user-data-dir=${join(temp,'chrome')}`,'about:blank'],{stdio:'ignore'});children.push(chrome);chrome.on('error',()=>{});
 const port=await until(async()=>String(await readFile(join(temp,'chrome','DevToolsActivePort'))).split('\n')[0],'Chrome did not start');
 const page=await (await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`,{method:'PUT'})).json();
 ws=new WebSocket(page.webSocketDebuggerUrl);await new Promise((r,j)=>{ws.onopen=r;ws.onerror=j;});let sequence=0;const pending=new Map();const requests=[];
 ws.onmessage=({data})=>{const m=JSON.parse(data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result);}if(m.method==='Network.requestWillBeSent')requests.push(m.params.request);};
 const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;pending.set(id,{resolve,reject});ws.send(JSON.stringify({id,method,params}));});
 const evaluate=async(expression)=>{const result=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true,userGesture:true});if(result.exceptionDetails)throw new Error(result.exceptionDetails.text);return result.result.value;};
 await call('Network.enable');await call('Page.enable');
 await until(()=>evaluate(`!!document.querySelector('[data-testid="start"]')`),'missing start control');
 await evaluate(`document.querySelector('[data-testid="start"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'run did not start');
 await evaluate(`document.querySelector('[data-testid="cancel"]').click()`);
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'cancel not persisted');
 await call('Page.reload');await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='cancelled'`),'cancel lost after reload');
 await evaluate(`document.querySelector('[data-testid="replay"]').click()`);const before=requests.length;
 await until(()=>evaluate(`document.querySelector('[data-testid="mode"]')?.textContent==='Recorded replay'`),'replay unavailable');
 await delay(250);assert.equal(requests.slice(before).filter(r=>r.method==='POST').length,0,'replay mutated server');
 await evaluate(`document.querySelector('[data-testid="start"]').click()`);await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='running'`),'second run did not start');
 await call('Network.emulateNetworkConditions',{offline:true,latency:0,downloadThroughput:-1,uploadThroughput:-1});await delay(400);await call('Network.emulateNetworkConditions',{offline:false,latency:0,downloadThroughput:-1,uploadThroughput:-1});
 await until(()=>evaluate(`document.querySelector('[data-testid="run-state"]')?.textContent==='succeeded'`),'run failed to reconnect',20000);
 const seq=await evaluate(`Array.from(document.querySelectorAll('[data-event-seq]'),e=>Number(e.dataset.eventSeq))`);assert(seq.length>12);assert.equal(new Set(seq).size,seq.length);assert(seq.every((n,i)=>i===0||n===seq[i-1]+1),'journal lost an event');
 const capture=await call('Page.captureScreenshot',{format:'png'});await writeFile(join(root,'docs/implementation/runtime/studio-browser.png'),Buffer.from(capture.data,'base64'));
 console.log(JSON.stringify({start:true,cancel:true,reload:true,reconnect:true,replay_zero_posts:true,events:seq.length}));
}finally{ws?.close();for(const child of children.reverse())child.kill('SIGKILL');await delay(200);await rm(temp,{recursive:true,force:true});}
