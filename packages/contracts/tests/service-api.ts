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

import type {ProviderProposalV2,ProviderObserverV2,ProviderRecoveryRequestV2,ProviderRecoveryResultV2,ProviderServicesV2} from '@paper-theater/contracts';
import {providerServicesV2,validateProviderProposalV2,validateProviderIssuedOperationV2,validateProviderRecoveryResultV2} from '@paper-theater/contracts';
export async function issueAndExecuteV2(services:RunServices,raw:unknown,observer:ProviderObserverV2):Promise<ProviderResultV2>{
 validateProviderProposalV2(raw);const p:ProviderProposalV2=raw;const v2:ProviderServicesV2=providerServicesV2(services);
 const issued=await v2.issue_provider_v2(p);validateProviderIssuedOperationV2(issued,p,observer);return v2.execute_provider_v2(issued);
}
export async function recoverV2(services:RunServices,request:ProviderRecoveryRequestV2,observer:ProviderObserverV2):Promise<ProviderRecoveryResultV2>{
 const receipt=await providerServicesV2(services).recover_provider_v2(request);validateProviderRecoveryResultV2(receipt,request,observer);return receipt;
}
// Existing implementations do not acquire three new required methods.
export function preserveOldImplementor(s:Omit<RunServices,'issue_provider_v2'|'execute_provider_v2'|'recover_provider_v2'>):RunServices{return s;}

// Archive validation and scoped accessors retain explicit matrix identities.
export const legacyChapterRequirements=requiredAcceptanceChecks('production_chapter','pt-acceptance-required-v1');

import type {AcceptanceVerificationPolicyV2,SourceSliceRelationshipV1,SourceSliceBindingV1} from '@paper-theater/contracts';
import {validateAcceptanceVerificationPolicyV2,validateSourceSliceRelationshipV1} from '@paper-theater/contracts';
export function readSourceSlice(policy:unknown,relationship:unknown):SourceSliceRelationshipV1{validateAcceptanceVerificationPolicyV2(policy);const trusted:AcceptanceVerificationPolicyV2=policy;validateSourceSliceRelationshipV1(relationship,trusted);return relationship;}
export function mappedSourceSummary(report:AcceptanceVerificationReport):SourceSliceBindingV1|null{return report.source_slice??null;}

import type {DeliveryAudioObservationV1,Pcm16WavInfo} from '@paper-theater/contracts';
import {validateDeliveryAudioObservationV1} from '@paper-theater/contracts';
export function deliveryObservation(raw:unknown):Pcm16WavInfo{validateDeliveryAudioObservationV1(raw);const audio:DeliveryAudioObservationV1=raw;return audio.format;}

import {type PilotReviewPolicyV1,type PilotReviewObservationV1,type PilotPlanStructureV1,validatePilotReviewPolicyV1,validatePilotReviewObservationV1,validatePilotPlanStructureV1} from '@paper-theater/contracts';
declare const pilotPolicy:PilotReviewPolicyV1;
declare const pilotObservation:PilotReviewObservationV1;
declare const pilotStructure:PilotPlanStructureV1;
validatePilotReviewPolicyV1(pilotPolicy);validatePilotReviewObservationV1(pilotObservation);validatePilotPlanStructureV1(pilotStructure);
