import type {Limits,Pins,StageInput} from './services.js';
import type {ProviderArtifactRefWire,ProviderProposalV2} from './provider-metadata.js';
import type {ProductionPurposeV1,ProductionOperationScopeV1} from './production-configuration.js';
import {validateProviderProposalV2} from './provider-metadata.js';
import {ContractError,isDigest,isId,isSample,validateFile,sha256} from './validate.js';
import {registeredSourceCatalogV1} from './registered-source-scope.js';
import {parseStrictJsonBytes} from './json-bytes.js';

export type ProductionCompilerScopeV1=Omit<ProductionOperationScopeV1,'unit_id'|'request_sha256'>;
export interface ProductionParentPolicyV1 {
 schema_version:1;id:string;accounting_scope_id:string;book_id:string;source_id:string;source_sha256:string;
 compiler_sha256:string;pins:Pins;authorization_proof_sha256:string;accounting_original_sha256s:string[];
 imported_receipt_sha256s:string[];limits:Limits;operations:ProductionCompilerScopeV1[];
}
export interface ProductionInputLineageV1 {
 artifact:ProviderArtifactRefWire;source_sha256:string;compiler_sha256?:string|null;producer_receipt_sha256:string;
}
export interface ProductionDerivationV1 {
 schema_version:1;parent_policy:ProviderArtifactRefWire['file'];scope_id:string;compiler_sha256:string;
 source_sha256:string;purpose:ProductionPurposeV1;input:StageInput;proposal:ProviderProposalV2;input_lineage:ProductionInputLineageV1[];
}

const encoder=new TextEncoder();
const limitKeys=['money_microusd','requests','tokens','repairs','wall_time_ms'] as const;
function need(ok:unknown,field:string,message:string):asserts ok {if(!ok)throw new ContractError('production_parent',field,message);}
function record(v:unknown,keys:string[],field:string,optional:string[]=[]):Record<string,unknown> {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),field,'expected object');
 const r=v as Record<string,unknown>;need(keys.every(k=>Object.hasOwn(r,k))&&Object.keys(r).every(k=>keys.includes(k)||optional.includes(k)),field,'missing or unknown field');return r;
}
const id=(v:unknown):v is string=>isId(v)&&encoder.encode(v).length<=256;
function limits(v:unknown):asserts v is Limits {
 const r=record(v,[...limitKeys],'limits');need(limitKeys.every(k=>isSample(r[k]))&&Number(r.repairs)<=0xffffffff&&Number(r.wall_time_ms)>0&&Number(r.wall_time_ms)<=300000,'limits','bounded unsigned limits required');
}
const within=(a:Limits,b:Limits)=>limitKeys.every(k=>a[k]<=b[k]);
function pins(v:unknown):asserts v is Pins {
 const p=record(v,['recipe_sha256','profile_sha256','profile_id'],'pins');need(isDigest(p.recipe_sha256)&&isDigest(p.profile_sha256)&&id(p.profile_id),'pins','invalid frozen pins');
}
function hashes(v:unknown,nonempty=false):asserts v is string[] {
 need(Array.isArray(v)&&(!nonempty||v.length>0)&&v.length<=256&&v.every(isDigest)&&new Set(v).size===v.length,'policy','unique bounded receipt hashes required');
}
function semantic(v:unknown):string {
 if(Array.isArray(v))return '['+v.map(semantic).join(',')+']';
 if(v!==null&&typeof v==='object')return '{'+Object.entries(v).filter(([,v])=>v!=null).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([k,v])=>JSON.stringify(k)+':'+semantic(v)).join(',')+'}';
 return JSON.stringify(v);
}
// Match Rust's decoded serialization: lineage None is null; file None is omitted.
function metadataBytes(value:unknown):number {
 return encoder.encode(JSON.stringify(value,(_key,v:unknown)=>{
  if(v!==null&&typeof v==='object'&&!Array.isArray(v)){
   const o=v as Record<string,unknown>;
   if(['path','sha256','bytes','media_type'].every(k=>Object.hasOwn(o,k))){const {dimensions,...rest}=o;return dimensions==null?rest:{...rest,dimensions};}
   if(['artifact','source_sha256','producer_receipt_sha256'].every(k=>Object.hasOwn(o,k)))return {...o,compiler_sha256:o.compiler_sha256??null};
  }
  return v;
 })).length;
}
function file(v:unknown):asserts v is ProviderArtifactRefWire['file'] {
 const f=record(v,['path','sha256','bytes','media_type'],'file',['dimensions']);
 need(typeof f.path==='string'&&isDigest(f.sha256)&&isSample(f.bytes)&&typeof f.media_type==='string'&&f.media_type.length>0,'file','invalid file identity');
 validateFile({...v as ProviderArtifactRefWire['file'],dimensions:(v as ProviderArtifactRefWire['file']).dimensions??undefined});
}
function artifact(v:unknown):asserts v is ProviderArtifactRefWire {
 const a=record(v,['id','file','provenance'],'artifact');need(id(a.id)&&['autonomous','imported','diagnostic','synthetic'].includes(String(a.provenance)),'artifact','invalid input identity/provenance');file(a.file);
}

/** Consistency only. Runtime independently authenticates persisted authority. */
export function validateProductionParentPolicyV1(value:unknown):asserts value is ProductionParentPolicyV1 {
 const p=record(value,['schema_version','id','accounting_scope_id','book_id','source_id','source_sha256','compiler_sha256','pins','authorization_proof_sha256','accounting_original_sha256s','imported_receipt_sha256s','limits','operations'],'policy');
 need(p.schema_version===1&&id(p.id)&&id(p.accounting_scope_id)&&isDigest(p.compiler_sha256)&&isDigest(p.authorization_proof_sha256),'policy','registered compiler and inherited authority identities required');
 pins(p.pins);limits(p.limits);hashes(p.accounting_original_sha256s,true);hashes(p.imported_receipt_sha256s);
 need(registeredSourceCatalogV1().sources.some(s=>s.book_id===p.book_id&&s.source_id===p.source_id&&s.canonical_source_sha256===p.source_sha256),'policy','unregistered source');
 need(Array.isArray(p.operations)&&p.operations.length>0&&p.operations.length<=64&&encoder.encode(JSON.stringify(p)).length<=65536,'policy','bounded operation inventory required');
 const ids=new Set<string>();
 for(const value of p.operations){
  const o=record(value,['id','stage','purpose','kind','context','model','recipe_sha256','secret_env','limits','allow_recovery'],'operations');
  const context=record(o.context,['provider','project_id','credential_context_id'],'context');limits(o.limits);
  need(id(o.id)&&id(o.model)&&Object.values(context).every(id)&&!ids.has(o.id)&&isDigest(o.recipe_sha256),'operations','unique exact compiler scope required');ids.add(o.id);
  need(['source','direction','pilot','speech','acoustic','visual_preflight','visuals','presentation','music_mix','choreography','review','package'].includes(String(o.stage))&&['research','direction','speech','transcription','acoustic_verification','image','segmentation','music','review','token_count'].includes(String(o.kind))&&['pilot','bulk'].includes(String(o.purpose))&&typeof o.allow_recovery==='boolean','operations','unknown stage/kind/purpose/recovery');
  need(typeof o.secret_env==='string'&&/^[A-Z_][A-Z_0-9]{0,127}$/.test(o.secret_env)&&within(o.limits,p.limits)&&o.limits.requests>0&&(o.purpose!=='pilot'||o.stage==='pilot'),'operations','scope cannot widen limits or relabel Pilot');
 }
}

/** Exact-byte audit validation; never an issuance or execution capability. */
export async function validateProductionDerivationV1(policy:unknown,value:unknown,policyBytes:Uint8Array,logicalRequestBytes:Uint8Array):Promise<ProductionDerivationV1> {
 validateProductionParentPolicyV1(policy);
 const d=record(value,['schema_version','parent_policy','scope_id','compiler_sha256','source_sha256','purpose','input','proposal','input_lineage'],'derivation');
 file(d.parent_policy);need(d.parent_policy.bytes<=65536&&d.parent_policy.media_type==='application/json'&&Array.isArray(d.input_lineage)&&d.input_lineage.length<=128&&metadataBytes(d)<=65536,'bounds','bounded private derivation required');
 need(policyBytes instanceof Uint8Array&&policyBytes.length===d.parent_policy.bytes&&await sha256(policyBytes)===d.parent_policy.sha256,'parent_policy','original policy bytes differ');
 let original:unknown;try{original=parseStrictJsonBytes(policyBytes);validateProductionParentPolicyV1(original);}catch{throw new ContractError('production_parent','parent_policy','invalid original policy bytes');}
 need(semantic(original)===semantic(policy),'parent_policy','original policy differs');
 validateProviderProposalV2(d.proposal);
 const input=record(d.input,['schema_version','run_id','stage','unit_id','epoch','revision','inputs','pins','mode','limits','library_revision'],'stage');pins(input.pins);limits(input.limits);
 need(input.schema_version===1&&id(input.run_id)&&id(input.unit_id)&&[input.epoch,input.revision,input.library_revision].every(isSample)&&Array.isArray(input.inputs),'stage','invalid stage identity');
 input.inputs.forEach(artifact);
 const scope=policy.operations.find(o=>o.id===d.scope_id);need(scope,'scope_id','unconfigured compiler scope');
 const proposal=d.proposal;
 need(logicalRequestBytes instanceof Uint8Array&&logicalRequestBytes.length===proposal.payload.file.bytes&&await sha256(logicalRequestBytes)===proposal.payload.file.sha256,'logical_request','exact request bytes differ');
 need(d.schema_version===1&&d.compiler_sha256===policy.compiler_sha256&&d.source_sha256===policy.source_sha256&&input.mode==='live'&&input.stage===scope.stage&&d.purpose===scope.purpose&&semantic(input.pins)===semantic(policy.pins)&&proposal.kind===scope.kind&&proposal.provider===scope.context.provider&&proposal.model===scope.model&&proposal.recipe_sha256===scope.recipe_sha256&&proposal.payload.provenance==='autonomous'&&within(input.limits,scope.limits)&&proposal.timeout_ms<=input.limits.wall_time_ms&&proposal.estimate.requests<=input.limits.requests&&proposal.estimate.money_microusd<=input.limits.money_microusd&&Number.isSafeInteger(proposal.estimate.input_tokens+proposal.estimate.output_tokens)&&proposal.estimate.input_tokens+proposal.estimate.output_tokens<=input.limits.tokens,'derivation','derived request differs from frozen compiler scope');
 const expected=new Map<string,ProviderArtifactRefWire>();
 for(const a of [...input.inputs,...proposal.inputs] as ProviderArtifactRefWire[]){const previous=expected.get(a.id);need(previous==null||semantic(previous)===semantic(a),'input_lineage','conflicting input identity');expected.set(a.id,a);}
 need(expected.size>0&&expected.size<=128&&d.input_lineage.length===expected.size,'input_lineage','complete nonempty input provenance required');
 const seen=new Set<string>();
 for(const v of d.input_lineage){
  const l=record(v,['artifact','source_sha256','producer_receipt_sha256'],'input_lineage',['compiler_sha256']);artifact(l.artifact);const a=l.artifact;
  need(!seen.has(a.id)&&expected.has(a.id)&&semantic(expected.get(a.id))===semantic(a)&&l.source_sha256===policy.source_sha256&&isDigest(l.producer_receipt_sha256)&&(l.compiler_sha256==null||isDigest(l.compiler_sha256))&&(a.provenance==='autonomous'?l.compiler_sha256===policy.compiler_sha256:a.provenance==='imported'&&policy.imported_receipt_sha256s.includes(l.producer_receipt_sha256)),'input_lineage','unregistered, diagnostic or stale input provenance');seen.add(a.id);
 }
 return value as ProductionDerivationV1;
}
