import {ContractError,isDigest,isSample,safePath,sha256} from './validate.js';
export interface PublicExportEntryV1 {path:string;sha256:string;bytes:number;media_type:string;}
export interface PublicExportAllowlistV1 {schema_version:1;entries:PublicExportEntryV1[];}
const PRIVATE_SEGMENTS=new Set(['private','raw','journal','accounting']);
function need(ok:unknown,field:string,message:string):asserts ok {if(!ok)throw new ContractError('export_boundary',field,message);}
function publicPath(p:string):boolean{return safePath(p)&&p.split('/').every(s=>!PRIVATE_SEGMENTS.has(s));}
/** Executing verifier: every requested path must be exactly allowlisted, safe,
 *  public, byte-identical to the bound digest, and not a known private record. */
export async function verifyPublicExport(allowlist:PublicExportAllowlistV1,files:Array<{path:string;data:Uint8Array}>,knownPrivateDigests:string[]):Promise<void> {
 need(allowlist&&allowlist.schema_version===1,'schema_version','expected public export allowlist version 1');
 need(Array.isArray(allowlist.entries)&&allowlist.entries.length>0,'entries','an empty allowlist exports nothing');
 for(const e of allowlist.entries){need(typeof e.path==='string'&&publicPath(e.path),'path','allowlist must name safe public files only');need(isDigest(e.sha256)&&isSample(e.bytes)&&typeof e.media_type==='string'&&!!e.media_type,e.path,'invalid allowlist entry');}
 for(const f of files){
  need(publicPath(f.path),f.path,'unsafe or private path can never export');
  const entry=allowlist.entries.find(e=>e.path===f.path);
  need(entry,f.path,'path is not allowlisted for export');
  const actual=await sha256(f.data);
  need(actual===entry.sha256&&f.data.length===entry.bytes,f.path,'stale inventory: bytes differ from the allowlisted digest');
  need(!knownPrivateDigests.includes(actual),f.path,'bytes match a known private record');
 }
}
