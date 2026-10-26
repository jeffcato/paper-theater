import {test} from 'node:test';
import assert from 'node:assert/strict';
import {EventCursor,terminal,eventRecord} from '../src/state.ts';
test('snapshot N + reconnect deduplicates and detects missing committed events',()=>{
 const cursor=new EventCursor('run-a',10);const event=(seq)=>({run_id:'run-a',seq,summary:'{"type":"task_completed","record":{"stage":"source"}}'});
 assert.equal(cursor.accept(event(11)),true);assert.equal(cursor.accept(event(11)),false);assert.equal(cursor.accept(event(12)),true);assert.throws(()=>cursor.accept(event(14)),/snapshot_required/);assert.throws(()=>cursor.accept({...event(13),run_id:'run-b'}),/wrong_run/);assert.equal(cursor.seq,12);
 assert.deepEqual(eventRecord(event(12)),{type:'task_completed',record:{stage:'source'}});assert.equal(terminal('waiting_provider'),false);assert.equal(terminal('cancelled'),true);
});
