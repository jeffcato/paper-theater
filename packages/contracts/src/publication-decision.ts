import {ContractError,isDigest,sha256} from './validate.js';
import type {AcceptanceCheckerIdentity} from './acceptance-report.js';
import {validateAcceptanceCoverageLedgerV1,type AcceptanceCoverageLedgerV1} from './acceptance-coverage.js';
import {verifyPublicExport,type PublicExportAllowlistV1} from './public-export-boundary.js';
import {validateTrustedPlayerLoadV1} from './player-consumer.js';
import type {AcceptanceScope} from './acceptance-matrix.js';
export interface AcceptedPublicationV1 {schema_version:1;report_sha256:string;checker:AcceptanceCheckerIdentity;scope:AcceptanceScope;matrix_version:string;allowlist_sha256:string;catalog_sha256:string;file_count:number;}
function need(ok:unknown,field:string,message:string):asserts ok {if(!ok)throw new ContractError('publication_decision',field,message);}
/** Executing verifier: binds an exact executed report to an exact payload.
 *  Never approves production; the runtime persists the returned record. */
export async function verifyAcceptedPublication(ledger:unknown,reportBytes:Uint8Array,expectedChecker:AcceptanceCheckerIdentity,allowlist:unknown,catalogSha256:string,files:Array<{path:string;data:Uint8Array}>,knownPrivate:string[]):Promise<AcceptedPublicationV1> {
 const valid=await validateAcceptanceCoverageLedgerV1(ledger,reportBytes,expectedChecker);
 need(isDigest(catalogSha256),'catalog','catalog digest must be a valid trusted identity');
 await verifyPublicExport(allowlist as PublicExportAllowlistV1,files,knownPrivate);
 const allow=(allowlist as PublicExportAllowlistV1).entries;
 need(allow.every(e=>files.some(f=>f.path===e.path)),'payload','allowlisted file missing from the published payload');
 need(files.some(f=>f.data.length===reportBytes.length&&f.data.every((b,i)=>b===reportBytes[i])),'report','published payload must carry the exact executed report bytes');
 need((await Promise.all(files.map(f=>sha256(f.data)))).includes(catalogSha256),'catalog','published catalog must travel inside the payload');
 return {schema_version:1,report_sha256:valid.report_sha256,checker:valid.checker,scope:valid.scope,matrix_version:valid.matrix_version,
  allowlist_sha256:await sha256(new TextEncoder().encode(JSON.stringify(allowlist))),catalog_sha256:catalogSha256,file_count:files.length};
}
/** Migration interface: authenticate a trusted load against an accepted ledger. */
export async function authenticateTrustedLoad(load:unknown,ledger:AcceptanceCoverageLedgerV1,catalogSha256:string):Promise<void> {
 const v=validateTrustedPlayerLoadV1(load);
 need(v.acceptance_report_sha256===ledger.report_sha256,'acceptance_report_sha256','load binds a different acceptance report');
 need(v.checker_sha256===ledger.checker.sha256,'checker_sha256','load binds a different executed checker');
 need(v.catalog_entry_sha256===catalogSha256,'catalog_entry_sha256','load binds a different published catalog');
}
