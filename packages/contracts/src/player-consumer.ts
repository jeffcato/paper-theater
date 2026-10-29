import {ContractError,isDigest,isId} from './validate.js';
/** Minimum trusted pinning a player consumer loads before genuine media exists. */
export interface TrustedPlayerLoadV1 {
 schema_version:1;profile_id:string;checker_sha256:string;acceptance_report_sha256:string;
 catalog_entry_sha256:string;release_id:string;allow_diagnostic:boolean;
}
export const ZERO_DIGEST='0000000000000000000000000000000000000000000000000000000000000000';
function need(ok:unknown,code:string,field:string,message:string):asserts ok {if(!ok)throw new ContractError(code,field,message);}
function shape(v:unknown):asserts v is TrustedPlayerLoadV1 {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),'shape','trusted_player_load','expected object');
 const keys=Object.keys(v as Record<string,unknown>);
 need(keys.length===7&&['schema_version','profile_id','checker_sha256','acceptance_report_sha256','catalog_entry_sha256','release_id','allow_diagnostic'].every(k=>keys.includes(k)),'shape','trusted_player_load','missing or unknown field');
}
/** Independent shared-verifier entry point. Grants no acceptance or media authority. */
export function validateTrustedPlayerLoadV1(v:unknown):TrustedPlayerLoadV1 {
 shape(v);
 need(v.schema_version===1,'version','schema_version','expected trusted player load version 1');
 need(isId(v.profile_id),'id','trusted_load.profile_id','expected a bounded ASCII identifier');
 need(isId(v.release_id),'id','trusted_load.release_id','expected a bounded ASCII identifier');
 for(const [field,value] of [['trusted_load.checker_sha256',v.checker_sha256],['trusted_load.acceptance_report_sha256',v.acceptance_report_sha256],['trusted_load.catalog_entry_sha256',v.catalog_entry_sha256]] as const){
  need(isDigest(value),'digest',field,'expected lowercase SHA-256');
  need(value!==ZERO_DIGEST,'trusted_identity',field,'trusted identities must never be zero-hash placeholders');
 }
 return v;
}
