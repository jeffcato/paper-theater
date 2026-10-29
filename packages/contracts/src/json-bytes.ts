import {ContractError} from './validate.js';

// JSON.parse collapses duplicate keys; Rust's strict records reject them.
// Scan the already syntax-checked text so escaped aliases cannot hide a duplicate.
export function parseStrictJsonBytes(bytes:Uint8Array):unknown {
 const text=new TextDecoder('utf-8',{fatal:true}).decode(bytes);
 const value:unknown=JSON.parse(text);let i=0;
 const space=()=>{while(/\s/.test(text[i]??'' )&&i<text.length)i++;};
 const fail=()=>{throw new ContractError('json_bytes','record','duplicate key or invalid JSON string');};
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
