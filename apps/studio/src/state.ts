import type {RunEvent,RunState} from '@paper-theater/contracts';
export class EventCursor {
 readonly run:string;seq:number;
 constructor(run:string,seq:number){this.run=run;this.seq=seq;}
 accept(event:Pick<RunEvent,'run_id'|'seq'>):boolean{
  if(event.run_id!==this.run)throw new Error('wrong_run');
  if(!Number.isSafeInteger(event.seq)||event.seq<1)throw new Error('snapshot_required');
  if(event.seq<=this.seq)return false;
  if(event.seq!==this.seq+1)throw new Error('snapshot_required');
  this.seq=event.seq;return true;
 }
}
export function terminal(state:RunState):boolean{return ['succeeded','failed','cancelled'].includes(state);}
export function eventRecord(event:Pick<RunEvent,'summary'>):{type:string;record:Record<string,unknown>}{
 try{const data=JSON.parse(event.summary);if(typeof data.type==='string'&&data.record&&typeof data.record==='object')return data;}catch{}
 return {type:'recorded_entry',record:{summary:event.summary}};
}
