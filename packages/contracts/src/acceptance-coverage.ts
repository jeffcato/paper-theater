import type {AcceptanceScope} from './acceptance-matrix.js';
import type {AcceptanceBinding,AcceptanceCheckerIdentity,AcceptanceCheckResult,AcceptanceVerificationReport} from './acceptance-report.js';
import {validateAcceptanceVerificationReport} from './acceptance-report.js';
import {ContractError,sha256} from './validate.js';

export type AcceptanceEvidenceOwnerV1='integration'|'generation'|'runtime'|'player'|'unassigned';
export interface AcceptanceCoverageRowV1 {evidence_owners:AcceptanceEvidenceOwnerV1[];result:AcceptanceCheckResult}
/** Exact-report metadata only; deserialization never grants publication authority. */
export interface AcceptanceCoverageLedgerV1 {
 schema_version:1;scope:AcceptanceScope;matrix_version:string;report_sha256:string;
 checker:AcceptanceCheckerIdentity;binding:AcceptanceBinding;rows:AcceptanceCoverageRowV1[];
 production_approved:false;blockers:string[];
}
function owners(id:string):AcceptanceEvidenceOwnerV1[] {
 if(['package-byte-integrity','evidence-binding-integrity','required-check-coverage'].includes(id))return ['integration'];
 if(id==='autonomous-provenance')return ['generation','runtime'];
 if(['publication-recovery','product-spend-authorization','public-export-boundary'].includes(id))return ['runtime'];
 if(['font-bundle','profile-r186-browser','target-device-budget','graphics-recovery-review','control-lifecycle-review','title-visibility-review'].includes(id))return ['player'];
 if(['rendered-creative-review','opening-action-capture-coverage','incoming-picture-readiness','required-action-accounting'].includes(id))return ['generation','player'];
 if(['complete-chapter','complete-excerpt','complete-source-coverage','initial-reference-research','image-scope-review','image-style-review','image-reference-review','image-extraction-review','image-stage-readability','independent-acoustic-evidence','narrator-continuity','speech-joins','music-bible','mix-intelligibility','story-coverage','full-audio-review-coverage','speech-fidelity','pilot-recording-review','primary-acoustic-evidence','delivery-fact-mapping','audio-format-review'].includes(id))return ['generation'];
 return ['unassigned'];
}
function canonical(v:unknown):string {
 if(Array.isArray(v))return '['+v.map(canonical).join(',')+']';
 if(v!==null&&typeof v==='object')return '{'+Object.entries(v).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([k,v])=>JSON.stringify(k)+':'+canonical(v)).join(',')+'}';
 return JSON.stringify(v);
}
// JSON.parse collapses duplicate keys; Rust's strict records reject them.
// Scan the already syntax-checked text so escaped aliases cannot hide a duplicate.
function strictReport(text:string):unknown {
 const value:unknown=JSON.parse(text);let i=0;
 const space=()=>{while(/\s/.test(text[i]??'' )&&i<text.length)i++;};
 const fail=()=>{throw new ContractError('acceptance_coverage','report','duplicate key or invalid JSON string');};
 const string=():string=>{
  const start=i++;
  while(i<text.length){if(text[i]==='\\'){i+=2;continue;}if(text[i++]==='"')break;}
  const s=JSON.parse(text.slice(start,i)) as string;
  if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/.test(s))fail();
  return s;
 };
 const walk=(depth:number):void=>{
  if(depth>128)fail();space();
  if(text[i]==='{'){
   i++;space();const seen=new Set<string>();if(text[i]==='}'){i++;return;}
   while(true){const key=string();if(seen.has(key))fail();seen.add(key);space();i++;walk(depth+1);space();if(text[i++]==='}')return;space();}
  }
  if(text[i]==='['){i++;space();if(text[i]===']'){i++;return;}while(true){walk(depth+1);space();if(text[i++]===']')return;}}
  if(text[i]==='"'){string();return;}
  while(i<text.length&&!/[\s,}\]]/.test(text[i]!))i++;
 };
 walk(0);return value;
}
/** Caller supplies independently selected exact report bytes and expected checker.
 * Validation cannot authenticate execution or reconstruct the Rust capability. */
export async function validateAcceptanceCoverageLedgerV1(value:unknown,reportBytes:Uint8Array,expectedChecker:AcceptanceCheckerIdentity):Promise<AcceptanceCoverageLedgerV1> {
 const report:unknown=strictReport(new TextDecoder('utf-8',{fatal:true}).decode(reportBytes));
 validateAcceptanceVerificationReport(report,expectedChecker);
 const r=report as AcceptanceVerificationReport;
 const expected:AcceptanceCoverageLedgerV1={schema_version:1,scope:r.scope,matrix_version:r.matrix_version,
  report_sha256:await sha256(reportBytes),checker:r.checker,binding:r.binding,
  rows:r.checks.map(result=>({evidence_owners:owners(result.check_id),result})),
  production_approved:false,blockers:r.blockers};
 if(canonical(value)!==canonical(expected))throw new ContractError('acceptance_coverage','ledger','coverage, ownership, outcome or exact report bytes differ');
 return value as AcceptanceCoverageLedgerV1;
}
