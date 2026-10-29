import catalog from './registered-source-scopes-v1.json' with {type:'json'};
import type {SourceSpan,Provenance} from './types.js';
import type {SourceSliceBindingV1} from './source-slice.js';
import {ContractError} from './validate.js';

export interface RegisteredSourceFrameV1 {scope_id:string;sha256:string;scalar_count:number;origin:number;chapter_span:SourceSpan}
export interface RegisteredChapterSourceV1 {book_id:string;source_id:string;provenance:Provenance;source_receipt_sha256:string;raw_sha256:string;canonical_source_sha256:string;canonical_scalar_count:number;frames:RegisteredSourceFrameV1[]}
export interface RegisteredSourceCatalogV1 {schema_version:1;version:string;sources:RegisteredChapterSourceV1[]}
export const REGISTERED_SOURCE_CATALOG_VERSION='pt-registered-source-scopes-v1';
export const REGISTERED_SOURCE_CATALOG_SHA256='865410192be03386b3ac16543aea02a24abd5ff0fb936c91c12d92b8e9fe6231';
export const REGISTERED_SOURCE_CATALOG_BYTES=2128;
export function registeredSourceCatalogV1():RegisteredSourceCatalogV1 {return structuredClone(catalog) as RegisteredSourceCatalogV1;}

/** Report metadata consistency. Native acceptance also executes all pinned reads. */
export function validateRegisteredSourceScopeMetadataV1(mapping:SourceSliceBindingV1,checkId:string):void {
 const found=catalog.sources.flatMap(source=>source.frames.map(frame=>({source,frame}))).find(x=>x.frame.scope_id===mapping.full_source_scope_id);
 const need=(ok:unknown)=>{if(!ok)throw new ContractError('acceptance_report','registered_source_scope','unregistered, incomplete or stale source extent');};
 need(found);const {source,frame}=found!;
 need(mapping.full_source_sha256===frame.sha256&&mapping.full_source_scalar_count===frame.scalar_count&&mapping.full_source_origin===frame.origin&&mapping.relationship_sha256!=null);
 const [a,b]=mapping.full_source_span;need(a>=frame.chapter_span[0]!&&b<=frame.chapter_span[1]!&&a<b);
 if(checkId==='complete-chapter')need(a===frame.chapter_span[0]&&b===frame.chapter_span[1]&&mapping.slice_sha256===source.canonical_source_sha256&&mapping.slice_scalar_count===source.canonical_scalar_count);
 need(checkId==='complete-chapter'||checkId==='complete-excerpt');
}
