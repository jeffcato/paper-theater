import assert from 'node:assert/strict';
import {sha256,verifyPublicExport} from '@paper-theater/contracts';
const enc=new TextEncoder();
const body=enc.encode('{"release":"oz-1"}');
const allowlist={schema_version:1,entries:[{path:'release.json',sha256:await sha256(body),bytes:body.length,media_type:'application/json'}]};
await verifyPublicExport(allowlist,[{path:'release.json',data:body}],[]);
await verifyPublicExport(allowlist,[],[]);
const bad=[['reports.json',body,'renamed private-content alias accepted'],
 ['private/receipt.json',enc.encode('{}'),'private root accepted'],
 ['raw/response.bin',enc.encode('{}'),'raw root accepted'],
 ['journal/entries.jsonl',enc.encode('{}'),'journal accepted'],
 ['accounting/ledger.json',enc.encode('{}'),'accounting accepted'],
 ['release/../private/receipt.json',enc.encode('{}'),'traversal accepted'],
 ['/absolute.json',enc.encode('{}'),'absolute path accepted'],
 ['release.json',enc.encode('{"release":"oz-2"}'),'stale inventory accepted']];
for(const [path,data,msg] of bad){let error;try{await verifyPublicExport(allowlist,[{path,data}],[]);}catch(e){error=e;}assert(error,msg);}
const secret=enc.encode('{"private_receipt":"sentinel"}');
const poisoned={schema_version:1,entries:[{path:'assets/receipt.svg',sha256:await sha256(secret),bytes:secret.length,media_type:'image/svg+xml'}]};
let error;try{await verifyPublicExport(poisoned,[{path:'assets/receipt.svg',data:secret}],[await sha256(secret)]);}catch(e){error=e;}
assert(error,'byte-identical private record accepted under an allowlisted name');
let version;try{await verifyPublicExport({schema_version:2,entries:allowlist.entries},[],[]);}catch(e){version=e;}
assert(version,'wrong schema version accepted');
let empty;try{await verifyPublicExport({schema_version:1,entries:[]},[],[]);}catch(e){empty=e;}
assert(empty,'empty allowlist accepted');
console.log('Public export boundary: 1 pass + 11 rejection classes agree in TypeScript');
