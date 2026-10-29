import type {Limits,Pins,Stage,OperationKind,StageInput} from './services.js';
import type {ProviderContext,ProviderProposalV2} from './provider-metadata.js';
import {validateProviderProposalV2} from './provider-metadata.js';
import {ContractError,isDigest,isId,isSample,validateFile} from './validate.js';

/** Administrative input only. Runtime's persisted authority is checked separately. */
export type ProductionPurposeV1='pilot'|'bulk';
export interface ProductionOperationScopeV1 {
 id:string;unit_id:string;stage:Stage;purpose:ProductionPurposeV1;kind:OperationKind;
 context:ProviderContext;model:string;recipe_sha256:string;request_sha256:string;
 secret_env:string;limits:Limits;allow_recovery:boolean;
}
export interface ProductionConfigurationV1 {
 schema_version:1;id:string;book_id:string;source_id:string;pins:Pins;
 authorization_proof_sha256:string;accounting_original_sha256s:string[];
 limits:Limits;operations:ProductionOperationScopeV1[];
}
const stages=['source','direction','pilot','speech','acoustic','visual_preflight','visuals','presentation','music_mix','choreography','review','package'];
const kinds=['research','direction','speech','transcription','acoustic_verification','image','segmentation','music','review','token_count'];
const limitKeys=['money_microusd','requests','tokens','repairs','wall_time_ms'] as const;
const encoder=new TextEncoder();
function need(ok:unknown,field:string,message:string):asserts ok {if(!ok)throw new ContractError('production_config',field,message);}
function record(v:unknown,keys:readonly string[],field:string):Record<string,unknown> {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),field,'expected object');
 const o=v as Record<string,unknown>;need(Object.keys(o).length===keys.length&&keys.every(k=>Object.hasOwn(o,k)),field,'missing or unknown fields');return o;
}
const id=(v:unknown)=>isId(v)&&encoder.encode(v).length<=256;
function limits(v:unknown,field:string):asserts v is Limits {
 const o=record(v,limitKeys,field);need(limitKeys.every(k=>isSample(o[k]))&&Number(o.repairs)<=0xffffffff,field,'invalid unsigned limits');
 need(Number(o.wall_time_ms)>0&&Number(o.wall_time_ms)<=300000,field,'expected bounded nonzero timeout');
}
function pins(v:unknown):asserts v is Pins {
 const p=record(v,['recipe_sha256','profile_sha256','profile_id'],'pins');
 need(isDigest(p.recipe_sha256)&&isDigest(p.profile_sha256)&&id(p.profile_id),'pins','invalid pins');
}
const within=(a:Limits,b:Limits)=>limitKeys.every(k=>a[k]<=b[k]);

export function validateProductionConfigurationV1(value:unknown):asserts value is ProductionConfigurationV1 {
 const c=record(value,['schema_version','id','book_id','source_id','pins','authorization_proof_sha256','accounting_original_sha256s','limits','operations'],'configuration');
 need(c.schema_version===1&&id(c.id),'schema_version','expected version 1 and a valid identity');
 pins(c.pins);limits(c.limits,'limits');
 need((c.book_id==='pg-55'&&c.source_id==='pg-55-chapter-1')||(c.book_id==='pg-36'&&c.source_id==='pg-36-book-1-chapter-1'),'source_id','book and selected chapter differ');
 need(isDigest(c.authorization_proof_sha256)&&Array.isArray(c.accounting_original_sha256s)&&c.accounting_original_sha256s.length>0&&c.accounting_original_sha256s.length<=256&&c.accounting_original_sha256s.every(isDigest)&&new Set(c.accounting_original_sha256s).size===c.accounting_original_sha256s.length,'accounting_original_sha256s','exact nonempty unique accounting and authorization identities required');
 need(Array.isArray(c.operations)&&c.operations.length>0&&c.operations.length<=64&&encoder.encode(JSON.stringify(value)).length<=65536,'operations','bounded nonempty operation inventory required');
 const ids=new Set<string>();
 for(const value of c.operations){
  const o=record(value,['id','unit_id','stage','purpose','kind','context','model','recipe_sha256','request_sha256','secret_env','limits','allow_recovery'],'operations');
  const context=record(o.context,['provider','project_id','credential_context_id'],'context');limits(o.limits,'operations.limits');
  need(id(o.id)&&id(o.unit_id)&&id(o.model)&&Object.values(context).every(id)&&!ids.has(o.id as string)&&isDigest(o.recipe_sha256)&&isDigest(o.request_sha256),'operations','unique operation and exact provider/model/recipe/request identities required');ids.add(o.id as string);
  need(stages.includes(o.stage as string)&&kinds.includes(o.kind as string)&&['pilot','bulk'].includes(o.purpose as string)&&typeof o.allow_recovery==='boolean','operations','unknown stage, kind, purpose or recovery flag');
  need(typeof o.secret_env==='string'&&/^[A-Z_][A-Z_0-9]{0,127}$/.test(o.secret_env),'secret_env','environment name only; credential contents are forbidden');
  need(within(o.limits,c.limits)&&o.limits.requests>0&&(o.purpose!=='pilot'||o.stage==='pilot'),'operations.limits','operation cannot widen limits or relabel Pilot scope');
 }
}

/** Consistency only; runtime still checks request bytes, durable scope and active epoch. */
export function validateConfiguredProviderProposalV1(config:unknown,scopeId:string,input:StageInput,proposal:unknown):void {
 validateProductionConfigurationV1(config);validateProviderProposalV2(proposal);
 pins(input.pins);limits(input.limits,'stage.limits');
 need(input.schema_version===1&&id(input.run_id)&&id(input.unit_id)&&[input.epoch,input.revision,input.library_revision].every(isSample),'stage','invalid stage scope');
 for(const a of input.inputs){need(id(a.id),'inputs','invalid artifact ID');validateFile({...a.file,...(a.file.dimensions==null?{dimensions:undefined}:{})});}
 const o=config.operations.find(o=>o.id===scopeId);need(o,'scope_id','operation is not configured');
 need(input.mode==='live'&&input.stage===o.stage&&input.unit_id===o.unit_id&&input.pins.recipe_sha256===config.pins.recipe_sha256&&input.pins.profile_sha256===config.pins.profile_sha256&&input.pins.profile_id===config.pins.profile_id&&within(input.limits,o.limits)&&proposal.kind===o.kind&&proposal.provider===o.context.provider&&proposal.model===o.model&&proposal.recipe_sha256===o.recipe_sha256&&proposal.payload.file.sha256===o.request_sha256&&proposal.timeout_ms<=input.limits.wall_time_ms&&proposal.estimate.requests<=input.limits.requests&&proposal.estimate.money_microusd<=input.limits.money_microusd&&Number.isSafeInteger(proposal.estimate.input_tokens+proposal.estimate.output_tokens)&&proposal.estimate.input_tokens+proposal.estimate.output_tokens<=input.limits.tokens,'proposal','request/stage/pins/estimate differ from configured scope');
}
