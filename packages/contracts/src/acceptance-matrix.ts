import v1 from './acceptance-required-v1.json' with {type:'json'};
import v2 from './acceptance-required-v2.json' with {type:'json'};
import v3 from './acceptance-required-v3.json' with {type:'json'};
import v6 from './acceptance-required-v6.json' with {type:'json'};
import {ContractError} from './validate.js';
export const ACCEPTANCE_MATRIX_VERSION='pt-acceptance-required-v6';
export type AcceptanceScope='diagnostic'|'autonomous_excerpt'|'production_chapter';
export interface RequiredAcceptanceCheck {id:string;scopes:AcceptanceScope[];target:string;implemented:boolean}
/** v1 stays immutable for archived receipts and explicitly pinned consumers. */
export function acceptanceCheckMatrix(version:string=ACCEPTANCE_MATRIX_VERSION):{version:string;checks:RequiredAcceptanceCheck[]}{
 const matrix=version==='pt-acceptance-required-v1'?v1:version==='pt-acceptance-required-v2'?v2:version==='pt-acceptance-required-v3'?v3:version==='pt-acceptance-required-v6'?v6:undefined;
 if(!matrix)throw new ContractError('acceptance_report','matrix_version','unsupported required matrix version');
 return structuredClone(matrix) as {version:string;checks:RequiredAcceptanceCheck[]};
}
export function requiredAcceptanceChecks(scope:AcceptanceScope,version:string=ACCEPTANCE_MATRIX_VERSION):RequiredAcceptanceCheck[]{return acceptanceCheckMatrix(version).checks.filter(c=>c.scopes.includes(scope));}
