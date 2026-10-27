"""Versioned offline PCM bundles. Original observations remain global and immutable."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import struct
import tempfile

FILE_LIMIT=16*1024*1024
TOTAL_LIMIT=64*1024*1024
META_LIMIT=1024*1024
OUTPUT_LIMIT=META_LIMIT
CHAPTER_LIMIT=256*1024*1024
COUNT_LIMIT=64
BLOCK=65536
ROOT=Path(__file__).resolve().parents[2]

def encoded(value):return (json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False)+'\n').encode()
def digest(b):return hashlib.sha256(b).hexdigest()
def recipe_digest():return digest(Path(__file__).with_name('chapter_helper.py').read_bytes()+Path(__file__).read_bytes())
def need(ok,message):
    if not ok:raise ValueError(message)
def safe(name):return isinstance(name,str) and bool(name) and not PurePosixPath(name).is_absolute() and all(p not in ('','..','.') for p in name.split('/'))
def location(root,name):
    need(safe(name),'unsafe bundle path');p=Path(root)
    for part in name.split('/'):
        p=p/part;need(not p.is_symlink(),'bundle symlink')
    need(p.resolve().is_relative_to(Path(root).resolve()),'bundle escape');return p

def ref(path,body,media='application/json'):return {'path':path,'sha256':digest(body),'bytes':len(body),'media_type':media}
def read(root,f,limit=FILE_LIMIT):
    need(type(f['bytes'])==int and 0<=f['bytes']<=limit,'input file limit')
    p=location(root,f['path']);need(p.stat().st_size==f['bytes'],'input length')
    with p.open('rb') as stream:b=stream.read(f['bytes']+1)
    need(len(b)==f['bytes'] and digest(b)==f['sha256'],'input hash');return b

def meta(root,name):
    p=location(root,name);need(p.stat().st_size<=META_LIMIT,'metadata limit')
    with p.open('rb') as stream:b=stream.read(META_LIMIT+1)
    need(len(b)<=META_LIMIT,'metadata limit');return json.loads(b),ref(name,b)

def header(rate,frames):
    need(type(rate)==int and 8000<=rate<=192000 and type(frames)==int and frames>0 and 44+frames*2<=CHAPTER_LIMIT,'PCM domain')
    return b'RIFF'+struct.pack('<I',36+frames*2)+b'WAVEfmt '+struct.pack('<IHHIIHH',16,1,1,rate,rate*2,2,16)+b'data'+struct.pack('<I',frames*2)
def native_header(b,size):
    need(len(b)>=44 and size>=46 and size<=CHAPTER_LIMIT,'PCM length')
    rate=struct.unpack('<I',b[24:28])[0];frames=(size-44)//2
    need(b[:44]==header(rate,frames) and size==44+frames*2,'canonical mono PCM16 required');return rate,frames

def stream_digest(path):
    h=hashlib.sha256();count=0
    with Path(path).open('rb') as stream:
        while b:=stream.read(BLOCK):
            count+=len(b);need(count<=CHAPTER_LIMIT,'chapter grew beyond envelope');h.update(b)
    return h.hexdigest()

def _contexts(index,a,b):
    result=[]
    for u in index['source_units']:
        x,y=u['frames']
        if x<b and a<y:result.append({'unit_id':u['id'],'source_span':u['source_span'],'global_frames':[x,y],'local_frames':[max(x,a)-a,min(y,b)-a],'complete':a<=x and y<=b})
    return result

def _requirements(index,fact,facts):
    if fact['outcome']!='pass':return []
    intervals={i['id']:i for i in facts['intervals']};offset=index['evidence_scope'][0];needed=set()
    for key in fact['interval_ids']:
        i=intervals[key];need(i['outcome']=='pass','unverified original interval')
        a=offset+min(i['primary_start_sample'],i['independent_start_sample']);b=offset+max(i['primary_end_sample'],i['independent_end_sample'])
        cursor=a
        for c in index['chunks']:
            x,y=c['frames']
            if x<b and a<y:need(x<=cursor,'uncovered interval');needed.add(c['id']);cursor=min(b,y)
        need(cursor==b,'interval out of chapter')
    return sorted(needed)

def _facts(index,facts,present):
    rows=[]
    for f in facts['facts']:
        required=_requirements(index,f,facts);bound=None;outcome=f['outcome'];reason='original_fact_unresolved'
        if outcome=='pass':
            if not index['complete_original_scope']:outcome='uncertain';reason='chapter_global_evidence_incomplete'
            elif not set(required)<=set(present):outcome='missing';reason='cross_cut_evidence_missing'
            else:bound=f['not_before_sample']+index['evidence_scope'][0];reason='all_global_supporting_chunks_verified'
        rows.append({'original':f,'required_chunks':required,'present_chunks':[c for c in required if c in present],'outcome':outcome,'chapter_not_before_sample':bound,'reason':reason})
    return rows

def _observations(items,a,b,offset):
    result=[]
    for item in items:
        x=item['start_sample']+offset;y=item['end_sample']+offset
        if (x==y and a<=x<b) or (x<y and x<b and a<y):
            result.append({'original':item,'global_samples':[x,y],'local_samples':[max(a,x)-a,min(b,y)-a],'complete':a<=x and y<=b})
    return result

def _context(root,index):
    files=index['context_files'];need(len(files)+4<=COUNT_LIMIT and len({f['path'] for f in files})==len(files),'context count/duplicates')
    need(all(type(f['bytes'])==int and 0<=f['bytes']<=FILE_LIMIT for f in files),'context file limit')
    need(sum(f['bytes'] for f in files)+4*META_LIMIT<TOTAL_LIMIT,'context aggregate')
    data={f['path']:read(root,f) for f in files}
    source=data['chapter-source.txt'].decode();original=data['source.txt'].decode();need(source==original,'source identity mismatch')
    manifest=json.loads(data['manifest.json']);normalized=json.loads(data['acoustic-replay.json']);facts=json.loads(data['fact-evidence.json']);inventory=json.loads(data['source-facts.json'])
    need(digest(data['source.txt'])==manifest['source_sha256']==facts['source_sha256']==inventory['source_sha256'],'source hashes')
    need(digest(data['excerpt.wav'])==manifest['audio_sha256']==facts['narration_sha256']==normalized['audio_sha256'],'narration hashes')
    need(facts['source_facts_sha256']==digest(data['source-facts.json']) and facts['normalized_report_sha256']==digest(data['acoustic-replay.json']),'stale original fact bindings')
    need(normalized['input_hashes']=={p:digest(data[p]) for p in normalized['input_hashes']},'stale original observations')
    rate,frames=native_header(data['excerpt.wav'],len(data['excerpt.wav']));need(index['sample_rate']==rate and index['evidence_scope'][1]-index['evidence_scope'][0]==frames,'evidence scope')
    units=[];receipts=[];base=manifest['source_span_characters'][0];cursor=base;offset=index['evidence_scope'][0]
    cached=json.loads(data['segmented-acoustic-report.json'])
    for u,saved in zip(manifest['units'],cached['units']):
        a,b=u['source_span_characters'];need(base<=a<b<=base+len(source) and a>=cursor and not source[cursor-base:a-base].strip(),'source unit coverage')
        need(source[a-base:b-base]==u['text'],'source unit text');cursor=b
        request=saved['request_sha256'];path=f'transcribe/{request}.receipt.json';r=json.loads(data[path])
        need(r['request_sha256']==request and r['audio_sha256']==digest(data[u['audio']]) and r['response_sha256']==digest(data[f'transcribe/{request}.json']),'primary receipt binding')
        units.append({'id':saved['case'],'source_span':[a,b],'frames':[offset+u['start_sample'],offset+u['end_sample']]})
        receipts.append({'path':path,'sha256':digest(data[path]),'model':r['model'],'request_sha256':request,'response_sha256':r['response_sha256'],'audio_sha256':r['audio_sha256'],'provider_recipe_sha256':r.get('harness_sha256'),'scope':'original response; chunk observations are derived'})
    need(not source[cursor-base:].strip(),'source tail coverage')
    for path in sorted(data):
        if path.startswith('meta/') and path.endswith('.receipt.json'):
            r=json.loads(data[path]);need(r['audio_sha256']==digest(data['excerpt.wav']) and r['response_sha256']==digest(data[f"meta/{r['request_sha256']}.json"]),'independent receipt binding')
            receipts.append({'path':path,'sha256':digest(data[path]),'model':r['model'],'request_sha256':r['request_sha256'],'response_sha256':r['response_sha256'],'audio_sha256':r['audio_sha256'],'provider_recipe_sha256':r.get('harness_sha256'),'scope':'original full-scope response; no chunk-local uniqueness claim'})
    need(index['source_origin']==base and index['source_units']==units and index['receipts']==receipts,'context declarations changed')
    need(index['source']['sha256']==digest(data['chapter-source.txt']),'chapter source hash')
    complete=index['evidence_scope']==[0,index['frame_count']] and index['narration']['sha256']==digest(data['excerpt.wav'])
    need(index['complete_original_scope']==complete,'unsupported global evidence scope')
    return data,normalized,facts

def _index(root):
    index,identity=meta(root,'chapter-index.json')
    need(index['schema_version']==1 and index['provenance']=='diagnostic' and index['recipe_sha256']==recipe_digest(),'bundle recipe/version')
    need(type(index['source_origin'])==int and all(type(n)==int and 0<=n<=9007199254740991 for n in index['evidence_scope']),'integer source/evidence coordinates')
    need(index['header_hex']==header(index['sample_rate'],index['frame_count']).hex() and index['narration']['bytes']==44+2*index['frame_count'],'chapter format')
    need(0<=index['evidence_scope'][0]<index['evidence_scope'][1]<=index['frame_count'],'evidence placement')
    need(1<=len(index['chunks'])<=1024,'chunk count');cursor=0;ids=set();paths=set()
    for c in index['chunks']:
        a,b=c['frames'];need(type(a)==int and type(b)==int,'integer chunk coordinates');need(a==cursor and a<b<=index['frame_count'] and c['id'] not in ids and c['file']['path'] not in paths,'ordered complete chunk coverage')
        need(c['file']['bytes']==44+(b-a)*2<=FILE_LIMIT and c['source_contexts']==_contexts(index,a,b),'chunk/source metadata');cursor=b;ids.add(c['id']);paths.add(c['file']['path'])
    need(cursor==index['frame_count'],'incomplete chapter');return index,identity

def verify_unit(root,unit_path):
    unit,unit_ref=meta(root,unit_path);index,index_ref=_index(root)
    need(unit['schema_version']==1 and unit['recipe_sha256']==recipe_digest() and unit['index']==index_ref,'unit binding')
    need(len(unit['files'])+3<=COUNT_LIMIT and sum(f['bytes'] for f in unit['files'])+3*META_LIMIT<=TOTAL_LIMIT,'unit aggregate/count')
    need(len({f['path'] for f in unit['files']})==len(unit['files']),'unit duplicates')
    chunks={c['id']:c for c in index['chunks']};need(unit['base_chunk'] in chunks and unit['base_chunk'] in unit['included_chunks'],'base chunk missing')
    expected=index['context_files']+[index_ref]+[chunks[c]['file'] for c in unit['included_chunks']]
    need(sorted(unit['files'],key=lambda f:f['path'])==sorted(expected,key=lambda f:f['path']),'unit file selection')
    data,normalized,facts=_context(root,index);checked={f['path']:f['sha256'] for f in unit['files']}
    original=memoryview(data['excerpt.wav'])[44:];scope_a,scope_b=index['evidence_scope'];verified=[]
    for key in unit['included_chunks']:
        c=chunks[key];b=read(root,c['file']);rate,frames=native_header(b,len(b));a,z=c['frames']
        need(rate==index['sample_rate'] and frames==z-a and digest(b[44:])==c['pcm_sha256'],'chunk PCM identity')
        lo,hi=max(a,scope_a),min(z,scope_b)
        if lo<hi:need(memoryview(b)[44+(lo-a)*2:44+(hi-a)*2]==original[(lo-scope_a)*2:(hi-scope_a)*2],'original PCM placement')
        verified.append(key)
    base=chunks[unit['base_chunk']];a,b=base['frames'];offset=index['evidence_scope'][0]
    report={'schema_version':1,'provenance':'diagnostic','production_approved':False,'provider_calls':0,'outcome':'needs_repair','recipe_sha256':recipe_digest(),'index_sha256':index_ref['sha256'],'unit_sha256':unit_ref['sha256'],'chapter_sha256':index['narration']['sha256'],'source_sha256':index['source']['sha256'],'base_chunk':base['id'],'global_frames':base['frames'],'checked_files':checked,'verified_chunks':verified,'words':_observations(normalized['word_track'],a,b,offset),'groups':_observations(normalized['highlight_groups'],a,b,offset),'facts':_facts(index,facts,verified),'reveal_permissions':[],'scope':'Derived chunk observations; full global transcripts retained. Original provider receipts are not chunk responses.'}
    need(len(encoded(report))<=OUTPUT_LIMIT,'unit output limit');return report

def verify_bundle(root):
    index,identity=_index(root);data,normalized,facts=_context(root,index)
    del data
    h=hashlib.sha256(bytes.fromhex(index['header_hex']));pcm_hash=hashlib.sha256();verified=[]
    for c in index['chunks']:
        b=read(root,c['file']);rate,frames=native_header(b,len(b));need(rate==index['sample_rate'] and frames==c['frames'][1]-c['frames'][0] and digest(b[44:])==c['pcm_sha256'],'chunk PCM identity')
        h.update(memoryview(b)[44:]);pcm_hash.update(memoryview(b)[44:]);verified.append(c['id']);del b
        unit_path=f"units/{c['id']}.json";actual=verify_unit(root,unit_path)
        record=location(root,f"records/{c['id']}.json")
        if record.exists():
            expected=encoded(actual);need(read(root,ref(f"records/{c['id']}.json",expected),OUTPUT_LIMIT)==expected,'stale helper record')
    need(h.hexdigest()==index['narration']['sha256'],'chapter PCM reconstruction hash')
    return {'schema_version':1,'provenance':'diagnostic','production_approved':False,'provider_calls':0,'outcome':'needs_repair','recipe_sha256':recipe_digest(),'index_sha256':identity['sha256'],'chapter_sha256':h.hexdigest(),'chapter_pcm_sha256':pcm_hash.hexdigest(),'complete_pcm_coverage':True,'word_count':len(normalized['word_track']),'highlight_group_count':len(normalized['highlight_groups']),'point_word_count':sum(w['start_sample']==w['end_sample'] for w in normalized['word_track']),'facts':_facts(index,facts,verified),'reveal_permissions':[],'scope':'Physical bundle verification plus imported global facts; runtime independently revalidates original G16 evidence.'}

def produce(narration,source,evidence_index,output,*,max_frames=None,evidence_offset=0,project_root=ROOT):
    narration=Path(narration);source=Path(source);output=Path(output)
    need(not output.exists(),'output already exists');need(not narration.is_symlink() and not source.is_symlink(),'source symlink')
    with narration.open('rb') as stream:rate,frames=native_header(stream.read(44),narration.stat().st_size)
    need(source.stat().st_size<=FILE_LIMIT,'source limit')
    with source.open('rb') as stream:source_bytes=stream.read(FILE_LIMIT+1)
    need(len(source_bytes)<=FILE_LIMIT,'source grew beyond envelope');source_bytes.decode()
    need(Path(evidence_index).stat().st_size<=META_LIMIT,'evidence index limit')
    with Path(evidence_index).open('rb') as stream:raw_index=stream.read(META_LIMIT+1)
    need(len(raw_index)<=META_LIMIT,'evidence index grew beyond envelope')
    entries=json.loads(raw_index);context=[];payloads={}
    selected=[e for e in entries if not e['artifact']['file']['path'].startswith('delivery/') and e['artifact']['file']['path']!='presentation-recipe.json']
    need(len(selected)+6<=COUNT_LIMIT and all(type(e['artifact']['file']['bytes'])==int and 0<=e['artifact']['file']['bytes']<=FILE_LIMIT for e in selected),'context file/count limit')
    need(sum(e['artifact']['file']['bytes'] for e in selected)+len(source_bytes)+4*META_LIMIT<TOTAL_LIMIT,'context aggregate limit')
    for e in selected:
        f=e['artifact']['file'];p=f['path']
        if p.startswith('delivery/') or p=='presentation-recipe.json':continue
        body=read(project_root,{**f,'path':e['source']});payloads[p]=body;context.append(f)
    need(len({f['path'] for f in context})==len(context),'duplicate evidence')
    need(source_bytes==payloads['source.txt'],'source identity mismatch')
    payloads['chapter-source.txt']=source_bytes;context.append(ref('chapter-source.txt',source_bytes,'text/plain'));context.sort(key=lambda f:f['path'])
    context_bytes=sum(f['bytes'] for f in context)
    cap=min(FILE_LIMIT-44,TOTAL_LIMIT-context_bytes-4*META_LIMIT-44)//2
    need(cap>0 and len(context)+5<=COUNT_LIMIT,'context cannot fit helper envelope')
    if max_frames is None:max_frames=cap
    need(type(max_frames)==int and 0<max_frames<=cap and (frames+max_frames-1)//max_frames<=1024,'segment limit')
    original=payloads['excerpt.wav'];original_rate,original_frames=native_header(original,len(original))
    need(original_rate==rate and type(evidence_offset)==int and 0<=evidence_offset and evidence_offset+original_frames<=frames,'original narration placement')
    manifest=json.loads(payloads['manifest.json']);cached=json.loads(payloads['segmented-acoustic-report.json'])
    units=[];receipts=[]
    for u,saved in zip(manifest['units'],cached['units']):
        units.append({'id':saved['case'],'source_span':u['source_span_characters'],'frames':[evidence_offset+u['start_sample'],evidence_offset+u['end_sample']]})
        path=f"transcribe/{saved['request_sha256']}.receipt.json";r=json.loads(payloads[path]);receipts.append({'path':path,'sha256':digest(payloads[path]),'model':r['model'],'request_sha256':r['request_sha256'],'response_sha256':r['response_sha256'],'audio_sha256':r['audio_sha256'],'provider_recipe_sha256':r.get('harness_sha256'),'scope':'original response; chunk observations are derived'})
    for path in sorted(payloads):
        if path.startswith('meta/') and path.endswith('.receipt.json'):
            r=json.loads(payloads[path]);receipts.append({'path':path,'sha256':digest(payloads[path]),'model':r['model'],'request_sha256':r['request_sha256'],'response_sha256':r['response_sha256'],'audio_sha256':r['audio_sha256'],'provider_recipe_sha256':r.get('harness_sha256'),'scope':'original full-scope response; no chunk-local uniqueness claim'})
    full_hash=stream_digest(narration)
    index={'schema_version':1,'provenance':'diagnostic','production_approved':False,'provider_calls':0,'outcome':'needs_repair','recipe_sha256':recipe_digest(),'source':ref('chapter-source.txt',source_bytes,'text/plain'),'source_origin':manifest['source_span_characters'][0],'narration':{'path':'chapter.wav','sha256':full_hash,'bytes':narration.stat().st_size,'media_type':'audio/wav'},'sample_rate':rate,'channels':1,'bits_per_sample':16,'frame_count':frames,'header_hex':header(rate,frames).hex(),'evidence_scope':[evidence_offset,evidence_offset+original_frames],'complete_original_scope':evidence_offset==0 and frames==original_frames and full_hash==digest(original),'context_files':context,'source_units':units,'receipts':receipts,'limits':{'file':FILE_LIMIT,'aggregate':TOTAL_LIMIT,'metadata':META_LIMIT,'output':OUTPUT_LIMIT,'context_bytes':context_bytes,'maximum_segment_frames':cap},'chunks':[]}
    output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chapter-bundle-',dir=output.parent) as temp:
        root=Path(temp)
        for path,b in payloads.items():p=location(root,path);p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
        with narration.open('rb') as stream:
            stream.seek(44)
            for number,a in enumerate(range(0,frames,max_frames)):
                z=min(a+max_frames,frames);name=f'chunk-{number:04}';path=f'chunks/{name}.wav';p=location(root,path);p.parent.mkdir(parents=True,exist_ok=True)
                h=hashlib.sha256();pcm_h=hashlib.sha256();head=header(rate,z-a);h.update(head)
                with p.open('wb') as out:
                    out.write(head);remaining=(z-a)*2
                    while remaining:
                        b=stream.read(min(BLOCK,remaining));need(bool(b),'truncated chapter');out.write(b);h.update(b);pcm_h.update(b);remaining-=len(b)
                index['chunks'].append({'id':name,'frames':[a,z],'file':{'path':path,'sha256':h.hexdigest(),'bytes':44+2*(z-a),'media_type':'audio/wav'},'pcm_sha256':pcm_h.hexdigest(),'source_contexts':_contexts(index,a,z)})
        b=encoded(index);need(len(b)<=META_LIMIT,'index limit');(root/'chapter-index.json').write_bytes(b);index_ref=ref('chapter-index.json',b)
        facts=json.loads(payloads['fact-evidence.json'])
        for base in index['chunks']:
            needed={base['id']}
            for f in facts['facts']:
                required=_requirements(index,f,facts)
                if base['id'] in required:needed.update(required)
            selected=[c for c in index['chunks'] if c['id'] in needed]
            files=context+[index_ref]+[c['file'] for c in selected]
            if sum(f['bytes'] for f in files)+3*META_LIMIT>TOTAL_LIMIT or len(files)+3>COUNT_LIMIT:selected=[base];files=context+[index_ref,base['file']]
            unit={'schema_version':1,'provenance':'diagnostic','recipe_sha256':recipe_digest(),'index':index_ref,'base_chunk':base['id'],'included_chunks':[c['id'] for c in selected],'files':sorted(files,key=lambda f:f['path'])}
            p=root/f"units/{base['id']}.json";p.parent.mkdir(exist_ok=True);b=encoded(unit);need(len(b)<=META_LIMIT,'unit metadata limit');p.write_bytes(b)
            report=verify_unit(root,f"units/{base['id']}.json");p=root/f"records/{base['id']}.json";p.parent.mkdir(exist_ok=True);p.write_bytes(encoded(report))
        report=verify_bundle(root);(root/'bundle-verification.json').write_bytes(encoded(report))
        os.rename(root,output)
    return index

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('operation',choices=['produce','verify']);p.add_argument('output');p.add_argument('--narration');p.add_argument('--source');p.add_argument('--evidence-index');p.add_argument('--max-frames',type=int);p.add_argument('--evidence-offset',type=int,default=0);a=p.parse_args()
    if a.operation=='produce':produce(a.narration,a.source,a.evidence_index,a.output,max_frames=a.max_frames,evidence_offset=a.evidence_offset)
    else:print(encoded(verify_bundle(Path(a.output))).decode(),end='')
