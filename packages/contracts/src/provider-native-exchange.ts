import type {ProviderArtifactRefWire} from './provider-metadata.js';
import {validateProviderOperationV2,validateProviderResultV2} from './provider-metadata.js';
import {ContractError,isDigest,isSample,sha256,validateFile} from './validate.js';
import {parseStrictJsonBytes} from './json-bytes.js';

export const MAX_NATIVE_PROVIDER_RECORD_BYTES=64*1024;
export const MAX_NATIVE_PROVIDER_WIRE_BYTES=1024*1024;
export const MAX_NATIVE_PROVIDER_RAW_BYTES=12*1024*1024;

type NativeFile=ProviderArtifactRefWire['file'];
/** Runtime-selected pins; JSON validation grants no dispatch/retry authority. */
export interface ProviderNativeExchangePolicyV1 {
 schema_version:1;issued_operation:NativeFile;codec_sha256:string;origin_id:string;path:string;
 wire_request:NativeFile;max_raw_response_bytes:number;
}
export interface ProviderNativeExchangeV1 {
 schema_version:1;issued_operation:NativeFile;codec_sha256:string;origin_id:string;path:string;
 wire_request:NativeFile;raw_response?:ProviderArtifactRefWire|null;
}
export interface ProviderNativeExchangeBytesV1 {
 issued_operation:Uint8Array;logical_request:Uint8Array;wire_request:Uint8Array;raw_response?:Uint8Array|null;
}
function need(ok:unknown,field:string,message:string):asserts ok {if(!ok)throw new ContractError('native_exchange',field,message);}
function record(v:unknown,required:string[],optional:string[],field:string):Record<string,unknown> {
 if(v===null||typeof v!=='object'||Array.isArray(v))throw new ContractError('shape',field,'expected object');
 const o=v as Record<string,unknown>;if(!required.every(k=>Object.hasOwn(o,k))||!Object.keys(o).every(k=>required.includes(k)||optional.includes(k)))throw new ContractError('shape',field,'missing or unknown field');return o;
}
function file(v:unknown,field:string):asserts v is NativeFile {
 const f=record(v,['path','sha256','bytes','media_type'],['dimensions'],field);
 need(typeof f.path==='string'&&isDigest(f.sha256)&&isSample(f.bytes)&&typeof f.media_type==='string'&&f.media_type.length>0,field,'invalid file identity');
 validateFile({...v as NativeFile,dimensions:(v as NativeFile).dimensions??undefined});
}
// Rust fills absent Option fields with None. Keep old omission/null wire compatibility.
function semantic(v:unknown):string {
 if(Array.isArray(v))return '['+v.map(semantic).join(',')+']';
 if(v!==null&&typeof v==='object')return '{'+Object.entries(v).filter(([,v])=>v!=null).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([k,v])=>JSON.stringify(k)+':'+semantic(v)).join(',')+'}';
 return JSON.stringify(v);
}
const normalizedFile=(f:NativeFile)=>({path:f.path,sha256:f.sha256,bytes:f.bytes,media_type:f.media_type,...(f.dimensions==null?{}:{dimensions:f.dimensions})});
async function verify(f:NativeFile,bytes:Uint8Array):Promise<void> {
 file(f,'file');if(!(bytes instanceof Uint8Array)||bytes.length!==f.bytes||await sha256(bytes)!==f.sha256)throw new ContractError('file_hash',f.path,'exact bytes differ');
}

/** Actual verified bytes are required; this is not provider-execution or review authority. */
export async function validateProviderNativeExchangeV1(value:unknown,expected:unknown,issued:unknown,result:unknown,bytes:ProviderNativeExchangeBytesV1):Promise<ProviderNativeExchangeV1> {
 validateProviderOperationV2(issued);validateProviderResultV2(result,issued);
 need(issued.recovery==null,'issued_operation.recovery','existing-job queries use the separate recovery boundary');
 const keys=['schema_version','issued_operation','codec_sha256','origin_id','path','wire_request'];
 const r=record(value,keys,['raw_response'],'record'),p=record(expected,[...keys,'max_raw_response_bytes'],[],'policy');
 need(r.schema_version===1&&p.schema_version===1,'schema_version','expected 1');
 need(isDigest(r.codec_sha256),'codec_sha256','expected pinned codec digest');
 if(typeof r.origin_id!=='string'||!/^[A-Za-z0-9_.:-]{1,128}$/.test(r.origin_id))throw new ContractError('id','origin_id','expected bounded origin identifier');
 need(typeof r.path==='string'&&r.path.startsWith('/')&&!r.path.startsWith('//')&&r.path.length<=2048&&/^[\x21-\x7e]+$/.test(r.path)&&!/[?#%\\]/.test(r.path)&&r.path.split('/').every(p=>p!=='.'&&p!=='..'),'path','fixed relative provider path required');
 file(r.issued_operation,'issued_operation');file(r.wire_request,'wire_request');file(p.issued_operation,'policy.issued_operation');file(p.wire_request,'policy.wire_request');
 need(r.issued_operation.bytes<=65536&&r.wire_request.bytes<=1048576&&r.issued_operation.media_type==='application/json'&&r.wire_request.media_type==='application/json'&&isSample(p.max_raw_response_bytes)&&p.max_raw_response_bytes>0&&p.max_raw_response_bytes<=12*1048576&&(p.max_raw_response_bytes<=1048576||issued.operation.kind==='speech'&&issued.operation.provider==='gemini'),'bounds','bounded metadata/request and selected response role required');
 need(keys.every(k=>semantic(r[k])===semantic(p[k])),'policy','native exchange differs from independently selected pins');
 need(semantic(r.raw_response??null)===semantic(result.result.raw_response??null),'raw_response','original raw response differs');
 const raw=result.result.raw_response;
 const metadata={...r,issued_operation:normalizedFile(r.issued_operation),wire_request:normalizedFile(r.wire_request),raw_response:raw==null?null:{id:raw.id,file:normalizedFile(raw.file),provenance:raw.provenance}};
 const policyMetadata={...p,issued_operation:normalizedFile(p.issued_operation),wire_request:normalizedFile(p.wire_request)};
 const encoder=new TextEncoder();need([metadata,policyMetadata].every(m=>encoder.encode(JSON.stringify(m)).length<=MAX_NATIVE_PROVIDER_RECORD_BYTES),'bounds','native metadata remains bounded to 64 KiB');
 await verify(r.issued_operation,bytes.issued_operation);
 let original:unknown;try{original=parseStrictJsonBytes(bytes.issued_operation);validateProviderOperationV2(original);}catch{throw new ContractError('native_exchange','issued_operation','invalid original issued operation bytes');}
 need(semantic(original)===semantic(issued),'issued_operation','original issued operation differs');
 await verify(issued.operation.payload.file,bytes.logical_request);
 await verify(r.wire_request,bytes.wire_request);
 need(semantic(r.raw_response??null)===semantic(result.result.raw_response??null),'raw_response','original raw response differs');
 if(result.result.raw_response!=null){need(bytes.raw_response!=null&&result.result.raw_response.file.bytes<=p.max_raw_response_bytes,'raw_response','native raw response missing or exceeds selected cap');await verify(result.result.raw_response.file,bytes.raw_response);}
 else need(bytes.raw_response==null,'raw_response','raw bytes were not declared');
 return value as ProviderNativeExchangeV1;
}
