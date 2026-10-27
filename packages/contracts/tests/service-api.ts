import type {CandidateFile, CandidateWrite, RunServices,ProviderOperationV2,ProviderResultV2,ProviderContext,ProviderQuotaScope,ProviderAttemptIdentity,ProviderArtifactRefWire} from '@paper-theater/contracts';
import {validateProviderOperationV2,validateProviderResultV2,providerQuotaScopeKey} from '@paper-theater/contracts';
// Compile through the package's exported declarations, as downstream workers do.
export async function parseCandidate(services: RunServices, candidate: CandidateFile): Promise<unknown> {
  const bytes: Uint8Array = await services.read_candidate(candidate);
  return JSON.parse(new TextDecoder().decode(bytes));
}

export const legacyWrite: CandidateWrite = {candidate_id:"json-1",relative_path:"report.json",media_type:"application/json",max_bytes:4096,provenance:"diagnostic"};
export const pngWrite: CandidateWrite = {...legacyWrite,relative_path:"image.png",media_type:"image/png",dimensions:[128,64]};

export function readProviderResult(operation:unknown,result:unknown):ProviderResultV2 {
 validateProviderOperationV2(operation);
 const issued:ProviderOperationV2=operation;
 validateProviderResultV2(result,issued);
 const context:ProviderContext=result.context;
 const identity:ProviderAttemptIdentity=result.identity;
 void context;void identity;
 return result;
}
export function scopeIdentity(scope:ProviderQuotaScope):Promise<string>{return providerQuotaScopeKey(scope);}

// The Rust wire decoder accepts missing nullable legacy fields in v2 envelopes.
export function nullOnlyUsageGuardIsInsufficient(operation:unknown,result:unknown):number|null {
 validateProviderResultV2(result,operation);
 if(result.result.usage!==null){
  // @ts-expect-error Missing usage remains unknown, so a null-only guard is insufficient.
  return result.result.usage.requests;
 }
 return null;
}
export function nullishUsageGuard(operation:unknown,result:unknown):number|null {
 validateProviderResultV2(result,operation);
 return result.result.usage!=null?result.result.usage.requests:null;
}
export const nullableProviderArtifact:ProviderArtifactRefWire={id:'payload-1',provenance:'diagnostic',file:{path:'request.json',sha256:'a'.repeat(64),bytes:2,media_type:'application/json',dimensions:null}};

import type {AcceptanceVerificationReport,AcceptanceCheckerIdentity} from '@paper-theater/contracts';
import {validateAcceptanceVerificationReport,requiredAcceptanceChecks} from '@paper-theater/contracts';
export function readPrivateVerification(raw:unknown,checker:AcceptanceCheckerIdentity):AcceptanceVerificationReport {
 validateAcceptanceVerificationReport(raw,checker);
 const privateOnly:true=raw.private;const neverApproval:false=raw.production_approved;void privateOnly;void neverApproval;
 return raw;
}
export const chapterRequirements=requiredAcceptanceChecks('production_chapter');
