import type {ProviderProposalV2,ProviderOperationV2,ProviderResultV2,ProviderRecoveryRequestV2,ProviderRecoveryResultV2} from './provider-metadata.js';
import type {Id,Digest,ArtifactRef,Provenance,FileRef,Finding} from './types.js';
export type ExecutionMode='fixture'|'replay'|'live';
export type Stage='source'|'direction'|'pilot'|'speech'|'acoustic'|'visual_preflight'|'visuals'|'presentation'|'music_mix'|'choreography'|'review'|'package';
export type RunState='queued'|'running'|'waiting_provider'|'recovering'|'succeeded'|'failed'|'cancelled';
export interface Pins {recipe_sha256:Digest;profile_sha256:Digest;profile_id:Id}
export interface Limits {money_microusd:number;requests:number;tokens:number;repairs:number;wall_time_ms:number}
export interface StageInput {schema_version:1;run_id:Id;stage:Stage;unit_id:Id;epoch:number;revision:number;inputs:ArtifactRef[];pins:Pins;mode:ExecutionMode;limits:Limits;library_revision:number}
export type StageOutcome='complete'|'needs_repair'|'failed'|'cancelled';
export interface Decision {id:Id;summary:string;inputs:ArtifactRef[];selected_ids:Id[];rejected_ids:Id[];supersedes:Id|null}
export interface CandidateFile {candidate_id:Id;file:FileRef;provenance:Provenance}
export type BindingKind='character_sprite'|'voice'|'subject_reference'|'art_profile'|'musical_bible'|'shell';
export interface LibraryUpdate {book_id:Id;entity_id:Id;binding:BindingKind;expected_revision:number;artifacts:ArtifactRef[]}
export interface LibraryQuery {book_id:Id;entity_ids:Id[];revision:number}
export interface LibraryBinding {book_id:Id;entity_id:Id;binding:BindingKind;revision:number;artifacts:ArtifactRef[]}
export interface PackageCandidate {candidate_id:Id;provenance:Provenance;payload_inventory:FileRef[];inventory_sha256:Digest;acceptance:CandidateFile;manifest:CandidateFile;required_artifacts:ArtifactRef[];findings:Finding[];pins:Pins}
export interface StageOutput {outcome:StageOutcome;artifacts:CandidateFile[];decisions:Decision[];findings:Finding[];cache_keys:Digest[];library_updates:LibraryUpdate[];package_candidate:PackageCandidate|null}
export interface Usage {requests:number;input_tokens:number;output_tokens:number;money_microusd:number}
export type OperationKind='research'|'direction'|'speech'|'transcription'|'acoustic_verification'|'image'|'segmentation'|'music'|'review'|'token_count';
export interface ProviderOperation {attempt_id:Id;unit_id:Id;kind:OperationKind;provider:Id;model:Id;recipe_sha256:Digest;request_sha256:Digest;payload:ArtifactRef;inputs:ArtifactRef[];estimate:Usage;timeout_ms:number}
export interface ProviderResult {attempt_id:Id;completion:'completed'|'known_rejection'|'ambiguous';request_sha256:Digest;response_sha256:Digest|null;raw_response:ArtifactRef|null;outputs:ArtifactRef[];usage:Usage|null;started_at:string;elapsed_ms:number;diagnostic:string}
export type HelperKind='decode_audio'|'align_audio'|'mix_audio'|'register_image'|'capture'|'validate_package';
export interface HelperOperation {id:Id;kind:HelperKind;inputs:ArtifactRef[];recipe_sha256:Digest;timeout_ms:number;max_output_bytes:number}
export interface HelperResult {id:Id;outputs:CandidateFile[];findings:Finding[]}
export interface CandidateWrite {dimensions?:[number,number];candidate_id:Id;relative_path:string;media_type:string;max_bytes:number;provenance:Provenance}
export type EventKind='task_started'|'task_completed'|'director_decision'|'artifact_ready'|'check_completed'|'repair_planned'|'cache_reused'|'provider_wait'|'budget_update'|'run_terminal';
export interface JournalProposal {kind:EventKind;task_id:Id|null;decision_id:Id|null;artifact_ids:Id[];summary:string}
export type RunEvent=JournalProposal&{schema_version:1;run_id:Id;seq:number;epoch:number;revision:number;timestamp:string};
export interface RunRequest {schema_version:1;source_id:Id;book_id:Id;mode:ExecutionMode;pins:Pins;limits:Limits}
export interface StageSnapshot {stage:Stage;unit_id:Id;outcome:StageOutcome|null;artifacts:ArtifactRef[]}
export interface RunSnapshot {schema_version:1;run_id:Id;request_sha256:Digest;epoch:number;revision:number;state:RunState;last_event_seq:number;stages:StageSnapshot[];findings:Finding[]}

/** Private generation/runtime boundary; this is not an audience HTTP API.
 * Candidate reads verify execution ownership, scratch path, metadata and bytes,
 * with cancellation/epoch checks before and after the read.
 */
export interface RunServices {
  read_artifact(artifact: ArtifactRef): Promise<Uint8Array>;
  read_candidate(candidate: CandidateFile): Promise<Uint8Array>;
  write_candidate(request: CandidateWrite, bytes: Uint8Array): Promise<CandidateFile>;
  emit(entry: JournalProposal): Promise<void>;
  check_cancelled(): Promise<void>;
  provider(operation: ProviderOperation): Promise<ProviderResult>;
  /** Optional for source compatibility; use providerServicesV2 for unavailable defaults. */
  issue_provider_v2?(proposal:ProviderProposalV2):Promise<ProviderOperationV2>;
  execute_provider_v2?(issued:ProviderOperationV2):Promise<ProviderResultV2>;
  recover_provider_v2?(request:ProviderRecoveryRequestV2):Promise<ProviderRecoveryResultV2>;
  helper(operation: HelperOperation): Promise<HelperResult>;
  library(query: LibraryQuery): Promise<LibraryBinding[]>;
}

/** Staged private boundary; runtime rechecks mode, persisted issuance and active observer. */
export interface ProviderServicesV2 {
 issue_provider_v2(proposal:ProviderProposalV2):Promise<ProviderOperationV2>;
 execute_provider_v2(issued:ProviderOperationV2):Promise<ProviderResultV2>;
 recover_provider_v2(request:ProviderRecoveryRequestV2):Promise<ProviderRecoveryResultV2>;
}
export class ProviderV2Unavailable extends Error {
 readonly code='provider_v2_unavailable';
 constructor(){super('provider-v2 service unavailable; no legacy fallback');this.name='ProviderV2Unavailable';}
}
/** Bind optional implementations; missing methods reject without legacy calls or side effects. */
export function providerServicesV2(services:RunServices):ProviderServicesV2 {
 return {
  async issue_provider_v2(p){if(!services.issue_provider_v2)throw new ProviderV2Unavailable();return services.issue_provider_v2(p);},
  async execute_provider_v2(p){if(!services.execute_provider_v2)throw new ProviderV2Unavailable();return services.execute_provider_v2(p);},
  async recover_provider_v2(p){if(!services.recover_provider_v2)throw new ProviderV2Unavailable();return services.recover_provider_v2(p);},
 };
}
