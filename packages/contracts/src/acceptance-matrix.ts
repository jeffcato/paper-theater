import matrix from './acceptance-required-v1.json' with {type:'json'};
export const ACCEPTANCE_MATRIX_VERSION='pt-acceptance-required-v1';
export type AcceptanceScope='diagnostic'|'autonomous_excerpt'|'production_chapter';
export interface RequiredAcceptanceCheck {id:string;scopes:AcceptanceScope[];target:string;implemented:boolean}
export function acceptanceCheckMatrix():{version:string;checks:RequiredAcceptanceCheck[]}{return structuredClone(matrix) as {version:string;checks:RequiredAcceptanceCheck[]};}
export function requiredAcceptanceChecks(scope:AcceptanceScope):RequiredAcceptanceCheck[]{return acceptanceCheckMatrix().checks.filter(c=>c.scopes.includes(scope));}
