import type {CandidateFile, RunServices} from '@paper-theater/contracts';
// Compile through the package's exported declarations, as downstream workers do.
export async function parseCandidate(services: RunServices, candidate: CandidateFile): Promise<unknown> {
  const bytes: Uint8Array = await services.read_candidate(candidate);
  return JSON.parse(new TextDecoder().decode(bytes));
}
