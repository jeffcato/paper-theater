import type {FileRef,SourceSpan} from './types.js';
import type {AcceptanceVerificationPolicy,AcceptanceBinding,AcceptanceInputRole} from './acceptance-report.js';
import {ContractError,isId,isDigest,isSample,validateFile} from './validate.js';
export type SourceSliceFileRef=Omit<FileRef,'dimensions'>&{dimensions?:[number,number]|null};
export interface FullSourcePinV1 {file:SourceSliceFileRef;scope_id:string;scalar_count:number;origin:number}
export interface AcceptanceVerificationPolicyV2 {schema_version:2;base:Omit<AcceptanceVerificationPolicy,'inputs'>&{inputs:{role:AcceptanceInputRole;file:SourceSliceFileRef}[]};full_source:FullSourcePinV1;full_source_span:SourceSpan;relationship?:SourceSliceFileRef|null}
export interface SourceSliceRelationshipV1 {schema_version:1;full_source:FullSourcePinV1;slice:SourceSliceFileRef;slice_scope_id:string;slice_scalar_count:number;full_source_span:SourceSpan;local_span:SourceSpan;local_origin:number;global_origin:number}
export interface SourceSliceBindingV1 {schema_version:1;full_source_sha256:string;full_source_scope_id:string;full_source_scalar_count:number;full_source_origin:number;slice_sha256:string;slice_scope_id:string;slice_scalar_count:number;full_source_span:SourceSpan;local_span:SourceSpan;local_origin:number;global_origin:number;relationship_sha256?:string|null}
const need=(v:unknown,f:string,m:string):void=>{if(!v)throw new ContractError('source_slice',f,m);};
function record(v:unknown,required:string[],optional:string[]=[],field='record'):Record<string,any>{
 if(v===null||typeof v!=='object'||Array.isArray(v))throw new ContractError('shape',field,'expected record');const r=v as Record<string,any>;
 if(!required.every(k=>Object.hasOwn(r,k))||!Object.keys(r).every(k=>required.includes(k)||optional.includes(k)))throw new ContractError('shape',field,'missing or unknown field');return r;
}
function file(v:unknown):asserts v is SourceSliceFileRef {
 const r=record(v,['path','sha256','bytes','media_type'],['dimensions'],'file');need(typeof r.path==='string'&&typeof r.sha256==='string'&&typeof r.media_type==='string'&&Number.isInteger(r.bytes)&&r.bytes>=0,'file','invalid file shape');
 if(r.dimensions!=null)need(Array.isArray(r.dimensions)&&r.dimensions.length===2&&r.dimensions.every((n:unknown)=>typeof n==='number'&&Number.isInteger(n)&&n>=0&&n<=4294967295),'file.dimensions','invalid dimensions');validateFile({...r,dimensions:r.dimensions??undefined} as FileRef);
}
function span(v:unknown):asserts v is SourceSpan {need(Array.isArray(v)&&v.length===2&&v.every(isSample)&&v[0]<v[1],'span','expected positive half-open safe scalar span');}
function full(v:unknown):asserts v is FullSourcePinV1 {const r=record(v,['file','scope_id','scalar_count','origin']);file(r.file);need(isId(r.scope_id)&&isSample(r.scalar_count)&&r.scalar_count>0&&isSample(r.origin)&&isSample(r.origin+r.scalar_count)&&r.file.bytes<=32*1024*1024,'full_source','invalid full-source identity/count/origin/bounds');}
function base(v:unknown):asserts v is AcceptanceVerificationPolicyV2['base'] {
 const p=record(v,['schema_version','scope','inputs','source_span','narration_sample_rate','narration_sample_count','source_scope_id','payload_inventory_sha256','asset_sha256s','player_version','candidate_checker_version','checker_sha256','additional_required_check_ids'],['device_id'],'base');
 need(p.schema_version===1&&['diagnostic','autonomous_excerpt','production_chapter'].includes(p.scope),'base','unsupported legacy policy version/scope');
 const roles=['source','narration','alignment','presentation_map','recipe','profile','player','fact_inventory','evidence_inventory'];need(Array.isArray(p.inputs)&&p.inputs.length===9,'base.inputs','expected nine roles');
 for(const i of p.inputs){const r=record(i,['role','file']);need(roles.includes(r.role),'base.inputs','unknown role');file(r.file);}
 need(roles.every(role=>p.inputs.filter((i:{role:string})=>i.role===role).length===1),'base.inputs','duplicate/missing role');
 need([p.source_scope_id,p.player_version,p.candidate_checker_version].every(isId)&&(p.device_id==null||isId(p.device_id)),'base.identities','invalid identity');
 need(isDigest(p.checker_sha256)&&isDigest(p.payload_inventory_sha256)&&Array.isArray(p.asset_sha256s)&&p.asset_sha256s.length<=2048&&p.asset_sha256s.every(isDigest),'base.hashes','invalid expected identity');span(p.source_span);
 need(isSample(p.narration_sample_rate)&&p.narration_sample_rate>0&&p.narration_sample_rate<=192000&&isSample(p.narration_sample_count)&&p.narration_sample_count>0&&Array.isArray(p.additional_required_check_ids)&&p.additional_required_check_ids.length<=128&&p.additional_required_check_ids.every(isId),'base.limits','invalid policy bounds');
}
export function validateSourceSliceBindingV1(v:unknown,b:Pick<AcceptanceBinding,'source_sha256'|'source_scope_id'|'source_span'>):asserts v is SourceSliceBindingV1 {
 const s=record(v,['schema_version','full_source_sha256','full_source_scope_id','full_source_scalar_count','full_source_origin','slice_sha256','slice_scope_id','slice_scalar_count','full_source_span','local_span','local_origin','global_origin'],['relationship_sha256'],'source_slice');
 need(s.schema_version===1&&isDigest(s.full_source_sha256)&&isDigest(s.slice_sha256)&&(s.relationship_sha256==null||isDigest(s.relationship_sha256))&&isId(s.full_source_scope_id)&&isId(s.slice_scope_id),'binding','invalid source relationship identity');span(s.full_source_span);span(s.local_span);
 const end=s.full_source_origin+s.full_source_scalar_count;
 need(isSample(s.full_source_origin)&&isSample(s.full_source_scalar_count)&&s.full_source_scalar_count>0&&isSample(end)&&isSample(s.slice_scalar_count)&&s.slice_scalar_count>0&&s.local_origin===0&&s.local_span[0]===0&&s.local_span[1]===s.slice_scalar_count&&s.global_origin===s.full_source_span[0]&&s.full_source_span[0]>=s.full_source_origin&&s.full_source_span[1]<=end&&s.full_source_span[1]-s.full_source_span[0]===s.slice_scalar_count,'coordinates','expected one complete contiguous scalar slice with explicit origins');
 need(s.slice_sha256===b.source_sha256&&s.slice_scope_id===b.source_scope_id&&s.local_span[0]===b.source_span[0]&&s.local_span[1]===b.source_span[1],'binding','slice differs from local candidate/source/fact domain');
 need(s.full_source_scope_id!==s.slice_scope_id||(s.full_source_sha256===s.slice_sha256&&s.full_source_span[0]===s.full_source_origin&&s.full_source_span[1]===end&&s.slice_scalar_count===s.full_source_scalar_count),'scope_id','one scope ID cannot alias different full/slice identities');
}
export function validateAcceptanceVerificationPolicyV2(v:unknown):asserts v is AcceptanceVerificationPolicyV2 {
 const p=record(v,['schema_version','base','full_source','full_source_span'],['relationship'],'policy');need(p.schema_version===2,'policy.schema_version','expected 2');base(p.base);full(p.full_source);span(p.full_source_span);if(p.relationship!=null){file(p.relationship);need(p.relationship.bytes<=1024*1024,'relationship','relationship exceeds 1 MiB');}
 const slice=p.base.inputs.find(i=>i.role==='source')!.file;
 validateSourceSliceBindingV1({schema_version:1,full_source_sha256:p.full_source.file.sha256,full_source_scope_id:p.full_source.scope_id,full_source_scalar_count:p.full_source.scalar_count,full_source_origin:p.full_source.origin,slice_sha256:slice.sha256,slice_scope_id:p.base.source_scope_id,slice_scalar_count:p.base.source_span[1]-p.base.source_span[0],full_source_span:p.full_source_span,local_span:p.base.source_span,local_origin:0,global_origin:p.full_source_span[0],relationship_sha256:p.relationship?.sha256},{source_sha256:slice.sha256,source_scope_id:p.base.source_scope_id,source_span:p.base.source_span});
}
const canonical=(v:unknown):unknown=>Array.isArray(v)?v.map(canonical):v!==null&&typeof v==='object'?Object.fromEntries(Object.entries(v).filter(([,v])=>v!=null).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([k,v])=>[k,canonical(v)])):v;
/** Metadata consistency only. Rust independently reads/verifies the actual bytes. */
export function validateSourceSliceRelationshipV1(v:unknown,expected:unknown):asserts v is SourceSliceRelationshipV1 {
 validateAcceptanceVerificationPolicyV2(expected);const r=record(v,['schema_version','full_source','slice','slice_scope_id','slice_scalar_count','full_source_span','local_span','local_origin','global_origin'],[],'relationship');
 full(r.full_source);file(r.slice);const slice=expected.base.inputs.find(i=>i.role==='source')!.file;
 const wanted={schema_version:1,full_source:expected.full_source,slice,slice_scope_id:expected.base.source_scope_id,slice_scalar_count:expected.base.source_span[1],full_source_span:expected.full_source_span,local_span:expected.base.source_span,local_origin:0,global_origin:expected.full_source_span[0]};
 need(JSON.stringify(canonical(r))===JSON.stringify(canonical(wanted)),'relationship','relationship differs from independent runtime pins');
}
