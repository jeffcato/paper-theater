import {ContractError,isDigest,isSample} from './validate.js';
export const MAX_PCM16_WAV_BYTES=256*1024*1024;
export const MAX_NATIVE_PCM16_WAV_BYTES=32*1024*1024;
export interface Pcm16WavInfo {encoding:number;channels:number;sample_rate:number;bits_per_sample:number;block_align:number;byte_rate:number;file_bytes:number;data_offset:number;data_bytes:number;frame_count:number}
export interface DeliveryAudioObservationV1 {schema_version:1;master_sha256:string;timeline_sha256:string;wav_label:boolean;format:Pcm16WavInfo;manifest_sample_rate:number;timeline_sample_rate:number;manifest_frame_count:number;timeline_frame_count:number}
function need(v:unknown,f:string,m:string):asserts v {if(!v)throw new ContractError('audio_format',f,m);}
function record(v:unknown,fields:string[],f:string):Record<string,unknown>{need(v!==null&&typeof v==='object'&&!Array.isArray(v),f,'expected record');const o=v as Record<string,unknown>;need(Object.keys(o).every(k=>fields.includes(k))&&fields.every(k=>Object.hasOwn(o,k)),f,'missing/unknown format field');return o;}
/** Metadata consistency only; the Rust reader checks actual hash-verified RIFF/WAVE bytes. */
export function validatePcm16WavInfo(value:unknown):asserts value is Pcm16WavInfo {
 const fields=['encoding','channels','sample_rate','bits_per_sample','block_align','byte_rate','file_bytes','data_offset','data_bytes','frame_count'];const o=record(value,fields,'format');need(fields.every(k=>isSample(o[k])),'format','invalid format integer');const v=value as Pcm16WavInfo;
 need(v.encoding===1&&[1,2].includes(v.channels)&&v.sample_rate>=8000&&v.sample_rate<=192000&&v.bits_per_sample===16&&v.block_align===v.channels*2&&v.byte_rate===v.sample_rate*v.block_align&&v.file_bytes>=44&&v.file_bytes<=MAX_PCM16_WAV_BYTES&&v.data_offset>=44&&v.data_offset%2===0&&v.data_bytes>0&&v.data_offset+v.data_bytes<=v.file_bytes&&v.frame_count*v.block_align===v.data_bytes&&(v.file_bytes<=MAX_NATIVE_PCM16_WAV_BYTES||(v.channels===2&&v.sample_rate===48000)),'format','invalid native PCM16 metadata');
}
export function validateDeliveryAudioObservationV1(value:unknown):asserts value is DeliveryAudioObservationV1 {
 const o=record(value,['schema_version','master_sha256','timeline_sha256','wav_label','format','manifest_sample_rate','timeline_sample_rate','manifest_frame_count','timeline_frame_count'],'delivery_audio');validatePcm16WavInfo(o.format);need(o.schema_version===1&&isDigest(o.master_sha256)&&isDigest(o.timeline_sha256)&&typeof o.wav_label==='boolean'&&['manifest_sample_rate','timeline_sample_rate'].every(k=>isSample(o[k])&&Number(o[k])>0&&Number(o[k])<=192000)&&['manifest_frame_count','timeline_frame_count'].every(k=>isSample(o[k])&&Number(o[k])>0),'delivery_audio','invalid observed delivery binding');
}
export function matchesDeliveryAudio(v:DeliveryAudioObservationV1):boolean{return v.wav_label&&v.format.channels===2&&v.format.sample_rate===48000&&v.manifest_sample_rate===48000&&v.timeline_sample_rate===48000&&v.format.frame_count===v.manifest_frame_count&&v.format.frame_count===v.timeline_frame_count;}
