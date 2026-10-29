import {validateProviderOperationV2,validateProviderResultV2} from './provider-metadata.js';
import type {ProviderOperationV2,ProviderResultV2} from './provider-metadata.js';
import {ContractError,isDigest,isId,sha256,validateFile} from './validate.js';
/** Binds a primary audio-only transcription record to its committed operation and recorded result. */
export interface PrimaryProvenanceV1 {
 schema_version:1;operation_attempt_id:string;operation_request_sha256:string;
 primary_request:{id:string;file:{path:string;sha256:string;bytes:number;media_type:string;dimensions?:[number,number]|null};provenance:string};
 result_response_sha256:string|null;
}
function need(ok:unknown,code:string,field:string,message:string):asserts ok {if(!ok)throw new ContractError(code,field,message);}
function shape(v:unknown):asserts v is PrimaryProvenanceV1 {
 need(v!==null&&typeof v==='object'&&!Array.isArray(v),'shape','provenance','expected object');
 const keys=Object.keys(v as Record<string,unknown>);
 need(keys.length===5&&['schema_version','operation_attempt_id','operation_request_sha256','primary_request','result_response_sha256'].every(k=>keys.includes(k)),'shape','provenance','missing or unknown field');
}
/** Independent shared-verifier entry point. Verifies, never issues or dispatches. */
export async function validatePrimaryProvenanceV1(v:unknown,operation:unknown,result:unknown,primaryBytes:Uint8Array):Promise<PrimaryProvenanceV1> {
 shape(v);
 need(v.schema_version===1,'version','schema_version','expected primary provenance version 1');
 need(isId(v.operation_attempt_id),'id','provenance.operation_attempt_id','expected a bounded ASCII identifier');
 need(isDigest(v.operation_request_sha256),'digest','provenance.operation_request_sha256','expected lowercase SHA-256');
 need(isId(v.primary_request.id),'id','artifact.id','expected a bounded ASCII identifier');
 validateFile({...v.primary_request.file,dimensions:v.primary_request.file.dimensions??undefined});
 const digest=await sha256(primaryBytes);
 need(primaryBytes.length===v.primary_request.file.bytes&&digest===v.primary_request.file.sha256,'primary_binding','primary_request','primary request bytes differ from the bound artifact');
 validateProviderOperationV2(operation);
 const op=operation as ProviderOperationV2;
 need(v.operation_attempt_id===op.operation.attempt_id,'primary_binding','operation_attempt_id','primary record names an uncommitted operation attempt');
 need(v.operation_request_sha256===op.operation.request_sha256,'primary_binding','operation_request_sha256','primary record binds a stale operation request');
 validateProviderResultV2(result,operation);
 const res=result as ProviderResultV2;
 need((res.result.response_sha256??null)===v.result_response_sha256,'primary_binding','result_response_sha256','primary record binds a stale or forged recorded response');
 return v;
}
