import type {CandidateFile, CandidateWrite, RunServices,ProviderOperationV2,ProviderResultV2,ProviderContext,ProviderQuotaScope,ProviderAttemptIdentity} from '@paper-theater/contracts';
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
