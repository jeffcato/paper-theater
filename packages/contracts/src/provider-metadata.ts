import type {ProviderOperation,ProviderResult,Usage} from './services.js';
import type {ArtifactRef} from './types.js';
import {ContractError,isDigest,isSample,validateFile,sha256} from './validate.js';

export const PROVIDER_METADATA_VERSION=2;
export const MAX_PROVIDER_RECORD_BYTES=64*1024;
export interface ProviderContext {provider:string;project_id:string;credential_context_id:string}
export type ProviderQuotaScope=
 | {kind:'provider';provider:string}
 | {kind:'project';provider:string;project_id:string}
 | {kind:'model';provider:string;project_id:string;model:string};
export interface QuotaAmounts {money_microusd:number;requests:number;tokens:number;repairs:number}
export interface ProviderQuotaWindow {window_id:string;scope:ProviderQuotaScope;start_at_ms:number;reset_at_ms:number;retry_at_ms?:number|null;capacity:QuotaAmounts;accounted:QuotaAmounts}
export interface CreativeRepairIdentity {repair_id:string;previous_operation_id:string;repair_index:number}
export interface TransportRetryIdentity {previous_attempt_id:string;previous_request_sha256:string;retry_index:number}
export interface ProviderAttemptIdentity {run_id:string;epoch:number;operation_id:string;repair?:CreativeRepairIdentity|null;transport_retry?:TransportRetryIdentity|null}
export type ProviderJobStatus='submitted'|'running'|'succeeded'|'failed'|'unknown';
export interface ExistingProviderJob {attempt_id:string;request_sha256:string;provider_job_id:string;status:ProviderJobStatus;observed_at_ms:number}
export interface ProviderOperationV2 {schema_version:2;operation:ProviderOperation;context:ProviderContext;identity:ProviderAttemptIdentity;quota_windows:ProviderQuotaWindow[];recovery?:ExistingProviderJob|null}
export interface ObservedProviderUsage {tokens?:number|null;requests?:number|null;input_tokens?:number|null;output_tokens?:number|null;money_microusd?:number|null}
export type ProviderRetryReason='rate_limited'|'quota_exhausted'|'repair_exhausted'|'transient_transport'|'provider_unavailable'|'unknown_completion';
export interface ProviderRetryTiming {reason:ProviderRetryReason;observed_at_ms:number;not_before_ms:number;retry_after_ms?:number|null;window_ids:string[]}
export type ProviderEpochDisposition='current'|'obsolete';
export interface ProviderResultV2 {schema_version:2;result:ProviderResult;context:ProviderContext;identity:ProviderAttemptIdentity;quota_windows:ProviderQuotaWindow[];observed_usage:ObservedProviderUsage;accounted_usage:QuotaAmounts;started_at_ms:number;recorded_at_ms:number;epoch_disposition:ProviderEpochDisposition;retry?:ProviderRetryTiming|null;recovery?:ExistingProviderJob|null}

type Check=(v:unknown,f:string)=>void;
function need(ok:unknown,code:string,field:string,message:string):asserts ok {if(!ok)throw new ContractError(code,field,message);}
const text:Check=(v,f)=>need(typeof v==='string','shape',f,'expected a string');
const integer:Check=(v,f)=>need(typeof v==='number'&&Number.isInteger(v)&&v>=0,'shape',f,'expected an unsigned integer');
const oneOf=(...values:string[]):Check=>(v,f)=>need(typeof v==='string'&&values.includes(v),'shape',f,'unknown enum value');
const list=(item:Check):Check=>(v,f)=>{need(Array.isArray(v),'shape',f,'expected an array');v.forEach((x,i)=>item(x,f+'.'+i));};
const nullable=(check:Check):Check=>(v,f)=>{if(v!=null)check(v,f);};
function object(v:unknown,f:string,required:Record<string,Check>,optional:Record<string,Check>={}):void {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),'shape',f,'expected a record');
 const o=v as Record<string,unknown>;
 need(Object.keys(o).every(k=>Object.hasOwn(required,k)||Object.hasOwn(optional,k)),'shape',f,'unknown record field');
 for(const [k,check] of Object.entries(required))check(o[k],f+'.'+k);
 for(const [k,check] of Object.entries(optional))if(Object.hasOwn(o,k))check(o[k],f+'.'+k);
}
const usageShape:Check=(v,f)=>object(v,f,{requests:integer,input_tokens:integer,output_tokens:integer,money_microusd:integer});
const amountsShape:Check=(v,f)=>object(v,f,{requests:integer,tokens:integer,repairs:integer,money_microusd:integer});
const fileShape:Check=(v,f)=>object(v,f,{path:text,sha256:text,bytes:integer,media_type:text},{dimensions:nullable((v,f)=>{need(Array.isArray(v)&&v.length===2,'shape',f,'expected two dimensions');v.forEach(x=>{integer(x,f);need(Number(x)<=4294967295,'shape',f,'dimension exceeds u32');});})});
const artifactShape:Check=(v,f)=>object(v,f,{id:text,file:fileShape,provenance:oneOf('autonomous','imported','synthetic','diagnostic')});
const contextShape:Check=(v,f)=>object(v,f,{provider:text,project_id:text,credential_context_id:text});
const identityShape:Check=(v,f)=>object(v,f,{run_id:text,epoch:integer,operation_id:text},{repair:nullable((v,f)=>object(v,f,{repair_id:text,previous_operation_id:text,repair_index:integer})),transport_retry:nullable((v,f)=>object(v,f,{previous_attempt_id:text,previous_request_sha256:text,retry_index:integer}))});
const scopeShape:Check=(v,f)=>{
 need(v!==null&&typeof v==='object','shape',f,'expected a scope');const kind=(v as Record<string,unknown>).kind;
 const base={kind:oneOf('provider','project','model'),provider:text};
 if(kind==='provider')object(v,f,base);else if(kind==='project')object(v,f,{...base,project_id:text});else if(kind==='model')object(v,f,{...base,project_id:text,model:text});else need(false,'shape',f,'unknown quota scope');
};
const windowShape:Check=(v,f)=>object(v,f,{window_id:text,scope:scopeShape,start_at_ms:integer,reset_at_ms:integer,capacity:amountsShape,accounted:amountsShape},{retry_at_ms:nullable(integer)});
const jobShape:Check=(v,f)=>object(v,f,{attempt_id:text,request_sha256:text,provider_job_id:text,status:oneOf('submitted','running','succeeded','failed','unknown'),observed_at_ms:integer});
const operationShape:Check=(v,f)=>object(v,f,{attempt_id:text,unit_id:text,kind:oneOf('research','direction','speech','transcription','acoustic_verification','image','segmentation','music','review','token_count'),provider:text,model:text,recipe_sha256:text,request_sha256:text,payload:artifactShape,inputs:list(artifactShape),estimate:usageShape,timeout_ms:integer});
const resultShape:Check=(v,f)=>object(v,f,{attempt_id:text,completion:oneOf('completed','known_rejection','ambiguous'),request_sha256:text,outputs:list(artifactShape),started_at:text,elapsed_ms:integer,diagnostic:text},{response_sha256:nullable(text),raw_response:nullable(artifactShape),usage:nullable(usageShape)});
const retryShape:Check=(v,f)=>object(v,f,{reason:oneOf('rate_limited','quota_exhausted','repair_exhausted','transient_transport','provider_unavailable','unknown_completion'),observed_at_ms:integer,not_before_ms:integer,window_ids:list(text)},{retry_after_ms:nullable(integer)});
function operationV2Shape(v:unknown):asserts v is ProviderOperationV2 {object(v,'record',{schema_version:integer,operation:operationShape,context:contextShape,identity:identityShape,quota_windows:list(windowShape)},{recovery:nullable(jobShape)});}
function resultV2Shape(v:unknown):asserts v is ProviderResultV2 {object(v,'record',{schema_version:integer,result:resultShape,context:contextShape,identity:identityShape,quota_windows:list(windowShape),observed_usage:(v,f)=>object(v,f,{},Object.fromEntries(['requests','tokens','input_tokens','output_tokens','money_microusd'].map(k=>[k,nullable(integer)]))),accounted_usage:amountsShape,started_at_ms:integer,recorded_at_ms:integer,epoch_disposition:oneOf('current','obsolete')},{retry:nullable(retryShape),recovery:nullable(jobShape)});}
/** Archive shape checks only: absent metadata is never authority for dispatch. */
export function validateLegacyProviderOperation(v:unknown):asserts v is ProviderOperation {operationShape(v,'record');}
export function validateLegacyProviderResult(v:unknown):asserts v is ProviderResult {resultShape(v,'record');}

const bytes=(s:string)=>new TextEncoder().encode(s).length;
function envelope(version:number,v:unknown){need(version===2,'version','schema_version','expected provider metadata version 2');need(bytes(JSON.stringify(v))<=MAX_PROVIDER_RECORD_BYTES,'bounds','record','provider record exceeds 64 KiB');}
function id(v:string,f:string){need(/^[A-Za-z0-9_.:-]{1,128}$/.test(v),'id',f,'expected a bounded ASCII identifier');}
function num(v:number,f:string){need(isSample(v),'integer',f,'expected a nonnegative JSON safe integer');}
function hash(v:string,f:string){need(isDigest(v),'digest',f,'expected lowercase SHA-256');}
function usage(v:Usage,f:string){for(const n of Object.values(v))num(n,f);need(isSample(v.input_tokens+v.output_tokens),'integer',f,'combined token count exceeds safe integer range');}
function artifact(a:ArtifactRef){id(a.id,'artifact.id');validateFile({...a.file,dimensions:a.file.dimensions??undefined});}
function scopeTuple(s:ProviderQuotaScope):string[]{return s.kind==='provider'?['provider',s.provider]:s.kind==='project'?['project',s.provider,s.project_id]:['model',s.provider,s.project_id,s.model];}
export async function providerQuotaScopeKey(s:ProviderQuotaScope):Promise<string>{scopeShape(s,'scope');for(const part of scopeTuple(s))id(part,'scope');return sha256(new TextEncoder().encode(JSON.stringify(scopeTuple(s))));}
function windows(values:ProviderQuotaWindow[],ctx:ProviderContext,model:string){
 need(values.length>0&&values.length<=16,'quota_scope','quota_windows','expected 1..16 applicable windows');const ids=new Set<string>(),tuples=new Set<string>();let project=false;
 for(const w of values){
  id(w.window_id,'quota_windows.window_id');const s=w.scope;const applicable=s.provider===ctx.provider&&(s.kind==='provider'||s.project_id===ctx.project_id&&(s.kind==='project'||s.model===model));project||=s.kind==='project'&&applicable;
  need(applicable,'quota_scope','quota_windows.scope','scope does not match provider/project/model');num(w.start_at_ms,'quota_windows.start_at_ms');num(w.reset_at_ms,'quota_windows.reset_at_ms');if(w.retry_at_ms!=null)num(w.retry_at_ms,'quota_windows.retry_at_ms');
  need(w.start_at_ms<w.reset_at_ms,'quota_window','quota_windows.reset_at_ms','expected a positive half-open UTC-millisecond window');const tuple=JSON.stringify([scopeTuple(s),w.start_at_ms,w.reset_at_ms]);need(!ids.has(w.window_id)&&!tuples.has(tuple),'quota_window','quota_windows','duplicate window ID or scope/interval');ids.add(w.window_id);tuples.add(tuple);
  for(const [name,v] of [['capacity',w.capacity],['accounted',w.accounted]] as const)for(const n of Object.values(v))num(n,'quota_windows.'+name);
 }
 need(project,'quota_scope','quota_windows','project quota scope is required; credentials do not create new capacity');
}
function identity(v:ProviderAttemptIdentity,attempt:string,request:string){
 id(v.run_id,'identity.run_id');id(v.operation_id,'identity.operation_id');num(v.epoch,'identity.epoch');
 if(v.repair){const r=v.repair;id(r.repair_id,'identity.repair.repair_id');id(r.previous_operation_id,'identity.repair.previous_operation_id');num(r.repair_index,'identity.repair.repair_index');need(r.repair_index>0&&r.previous_operation_id!==v.operation_id,'repair','identity.repair','creative repair needs a new logical operation and positive repair index');}
 if(v.transport_retry){const r=v.transport_retry;id(r.previous_attempt_id,'identity.transport_retry.previous_attempt_id');hash(r.previous_request_sha256,'identity.transport_retry.previous_request_sha256');num(r.retry_index,'identity.transport_retry.retry_index');need(r.retry_index>0&&r.previous_attempt_id!==attempt,'retry','identity.transport_retry','transport retry needs a distinct previous attempt and positive index');need(r.previous_request_sha256===request,'provider_binding','identity.transport_retry','transport retry cannot change request bytes');}
}
function recovery(j:ExistingProviderJob,attempt:string,request:string){id(j.attempt_id,'recovery.attempt_id');hash(j.request_sha256,'recovery.request_sha256');need(j.attempt_id===attempt&&j.request_sha256===request,'recovery','recovery','recovery must identify this existing attempt/request');need(/^[!-~]{1,256}$/.test(j.provider_job_id),'recovery','recovery.provider_job_id','expected a bounded opaque printable ASCII job ID');num(j.observed_at_ms,'recovery.observed_at_ms');}

export function validateProviderOperationV2(v:unknown):asserts v is ProviderOperationV2 {
 operationV2Shape(v);envelope(v.schema_version,v);const c=v.context,op=v.operation;
 id(c.provider,'context.provider');id(c.project_id,'context.project_id');id(c.credential_context_id,'context.credential_context_id');need(c.provider===op.provider,'provider_binding','context.provider','provider identity differs');
 for(const [k,s]of Object.entries({attempt_id:op.attempt_id,unit_id:op.unit_id,provider:op.provider,model:op.model}))id(s,'operation.'+k);
 hash(op.recipe_sha256,'operation.recipe_sha256');hash(op.request_sha256,'operation.request_sha256');usage(op.estimate,'operation.estimate');need(op.estimate.requests>0,'usage','operation.estimate.requests','zero money does not remove request accounting');need(op.timeout_ms>0&&op.timeout_ms<=300000,'bounds','operation.timeout_ms','expected 1..300000 milliseconds');artifact(op.payload);need(op.payload.file.sha256===op.request_sha256,'provider_binding','operation.payload','payload and request must bind the same exact bytes');need(op.payload.file.bytes<=1024*1024&&op.inputs.length<=64,'bounds','operation','payload or input inventory exceeds bounds');op.inputs.forEach(artifact);identity(v.identity,op.attempt_id,op.request_sha256);if(v.recovery)recovery(v.recovery,op.attempt_id,op.request_sha256);windows(v.quota_windows,c,op.model);
}
const same=(a:unknown,b:unknown)=>{
 const canonical=(v:unknown):unknown=>Array.isArray(v)?v.map(canonical):v!==null&&typeof v==='object'?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b)).filter(([,x])=>x!=null).map(([k,x])=>[k,canonical(x)])):v;
 return JSON.stringify(canonical(a))===JSON.stringify(canonical(b));
};
export function validateProviderResultV2(v:unknown,operation:unknown):asserts v is ProviderResultV2 {
 validateProviderOperationV2(operation);resultV2Shape(v);envelope(v.schema_version,v);const op=operation,r=v.result;
 need(same(v.context,op.context)&&same(v.identity,op.identity)&&r.attempt_id===op.operation.attempt_id&&r.request_sha256===op.operation.request_sha256,'provider_binding','result','result must match the runtime-issued context, logical operation, attempt and request');num(v.started_at_ms,'started_at_ms');num(v.recorded_at_ms,'recorded_at_ms');num(r.elapsed_ms,'result.elapsed_ms');need(v.started_at_ms<=v.recorded_at_ms,'retry','recorded_at_ms','result recording predates start');need(r.started_at.length>0&&bytes(r.started_at)<=64&&!/\p{Cc}/u.test(r.started_at)&&bytes(r.diagnostic)<=4096&&r.outputs.length<=64,'bounds','result','result text or output inventory exceeds bounds');
 if(r.response_sha256!=null)hash(r.response_sha256,'result.response_sha256');if(r.raw_response){artifact(r.raw_response);need(r.response_sha256===r.raw_response.file.sha256,'provider_binding','result.raw_response','raw response digest differs');}r.outputs.forEach(artifact);need(r.completion!=='known_rejection'||r.outputs.length===0,'provider_binding','result.outputs','known rejection cannot deliver outputs');for(const n of Object.values(v.accounted_usage))num(n,'accounted_usage');
 const obs=v.observed_usage;for(const key of ['requests','tokens','input_tokens','output_tokens','money_microusd']as const)if(obs[key]!=null)num(obs[key],'observed_usage');
 if(obs.input_tokens!=null&&obs.output_tokens!=null)need(isSample(obs.input_tokens+obs.output_tokens)&&obs.tokens===obs.input_tokens+obs.output_tokens,'usage','observed_usage.tokens','aggregate and split observations differ');
 if(obs.tokens!=null)need((obs.input_tokens??0)+(obs.output_tokens??0)<=obs.tokens,'usage','observed_usage.tokens','known token components exceed observed aggregate');
 for(const key of ['requests','tokens','money_microusd']as const){const actual=obs[key],charged=v.accounted_usage[key],reserved=key==='tokens'?op.operation.estimate.input_tokens+op.operation.estimate.output_tokens:op.operation.estimate[key];need(charged>=(actual??0)&&(!(r.completion==='ambiguous'||actual==null)||charged>=reserved),'usage','accounted_usage','unknown or ambiguous usage must retain at least the original reservation; observed overages remain charged');}
 need(v.accounted_usage.tokens>=(obs.input_tokens??0)+(obs.output_tokens??0),'usage','accounted_usage.tokens','known token observations exceed retained charges');need(v.accounted_usage.repairs<=1&&(op.identity.repair!=null||v.accounted_usage.repairs===0),'usage','accounted_usage.repairs','an attempt records at most one distinct creative-repair consumption');
 const allKnown=obs.requests!=null&&obs.input_tokens!=null&&obs.output_tokens!=null&&obs.money_microusd!=null;
 need((r.usage!=null)===allKnown,'usage','result.usage','legacy usage is present exactly when every observed metric is known');if(r.usage){usage(r.usage,'result.usage');need(['requests','input_tokens','output_tokens','money_microusd'].every(key=>v.observed_usage[key as keyof ObservedProviderUsage]===r.usage?.[key as keyof Usage]),'usage','result.usage','legacy and per-metric observations differ');}
 windows(v.quota_windows,v.context,op.operation.model);need(v.quota_windows.length===op.quota_windows.length,'quota_window','quota_windows','result must retain each original quota window');
 for(const old of op.quota_windows){const w=v.quota_windows.find(w=>w.window_id===old.window_id);need(w,'quota_window','quota_windows','original quota window missing');need(same(w.scope,old.scope)&&w.start_at_ms===old.start_at_ms&&w.reset_at_ms===old.reset_at_ms&&same(w.capacity,old.capacity),'quota_window','quota_windows','original window identity/bounds/capacity changed');need(old.retry_at_ms==null||w.retry_at_ms!=null&&w.retry_at_ms>=old.retry_at_ms,'retry','quota_windows.retry_at_ms','persisted retry timestamp cannot be cleared or shortened');need(w.accounted.money_microusd>=v.accounted_usage.money_microusd&&w.accounted.requests>=v.accounted_usage.requests&&w.accounted.tokens>=v.accounted_usage.tokens&&w.accounted.repairs>=v.accounted_usage.repairs,'usage','quota_windows.accounted','window accounting omits this attempt’s conservative charges');}
 if(v.retry){const t=v.retry;num(t.observed_at_ms,'retry');num(t.not_before_ms,'retry');need(t.observed_at_ms<=v.recorded_at_ms&&t.not_before_ms>=t.observed_at_ms,'retry','retry','invalid observation/retry chronology');if(t.retry_after_ms!=null){num(t.retry_after_ms,'retry.retry_after_ms');need(t.observed_at_ms+t.retry_after_ms<=t.not_before_ms,'retry','retry.not_before_ms','retry deadline shortens the observed relative delay');}const ids=new Set(t.window_ids);need(ids.size===t.window_ids.length&&ids.size<=16&&[...ids].every(id=>v.quota_windows.some(w=>w.window_id===id)),'retry','retry.window_ids','unknown or duplicate retry window');need(!['quota_exhausted','repair_exhausted','rate_limited'].includes(t.reason)||ids.size>0,'retry','retry.window_ids','quota/rate/repair reason requires its applicable window');need(v.quota_windows.every(w=>w.retry_at_ms==null||t.not_before_ms>=w.retry_at_ms),'retry','retry.not_before_ms','retry deadline shortens a persisted window cooldown');}
 if(v.recovery){recovery(v.recovery,r.attempt_id,r.request_sha256);need(v.recovery.observed_at_ms<=v.recorded_at_ms,'recovery','recovery.observed_at_ms','job observation postdates this record');}if(op.recovery){const j=v.recovery,old=op.recovery;need(j&&j.provider_job_id===old.provider_job_id&&j.observed_at_ms>=old.observed_at_ms&&(!['succeeded','failed'].includes(old.status)||j.status===old.status),'recovery','recovery','existing job identity/observation or terminal state changed');}
}
