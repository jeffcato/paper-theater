"""Offline bounded JSON/file helper; stdout contains one response envelope."""
import hashlib
import io
import json
from pathlib import Path
import sys
import wave
from adapters import compare_text,tokens,word_check,parse_words,parse_turns,candidate_track,highlight_groups

MAX_BYTES=16*1024*1024

def digest(b): return hashlib.sha256(b).hexdigest()

def read(root,name):
    path=(root/name).resolve()
    if not path.is_relative_to(root) or path.stat().st_size>MAX_BYTES:raise ValueError('file outside envelope or too large')
    return path.read_bytes()

def verified(root,name,expected):
    b=read(root,name)
    if digest(b)!=expected:raise ValueError('artifact hash mismatch')
    return b

def load(root,name):return json.loads(read(root,name))

def replay(root):
    m=load(root,'manifest.json'); saved=load(root,'segmented-acoustic-report.json')
    verified(root,'manifest.json',saved['input_manifest_sha256'])
    source=verified(root,'source.txt',m['source_sha256']).decode()
    audio=verified(root,'excerpt.wav',m['audio_sha256'])
    with wave.open(io.BytesIO(audio)) as f:
        if (f.getframerate(),f.getnchannels(),f.getsampwidth())!=(m['sample_rate'],1,2):raise ValueError('audio format mismatch')
        duration=f.getnframes()/f.getframerate()
    cases=[];results=[];identities=[]
    if len(saved['units'])!=len(m['units']):raise ValueError('incomplete source turns')
    for unit,record in zip(m['units'],saved['units']):
        case=f"turn-{unit['turn']:03}"
        if record['case']!=case:raise ValueError('source turn order changed')
        verified(root,unit['audio'],unit['audio_sha256'])
        rid=record['request_sha256']; receipt=load(root,f'transcribe/{rid}.receipt.json')
        if receipt['request_sha256']!=rid or receipt['audio_sha256']!=unit['audio_sha256'] or receipt['model']!=saved['model']:raise ValueError('receipt binding mismatch')
        words=parse_words(json.loads(verified(root,f'transcribe/{rid}.json',receipt['response_sha256'])))
        word_check(words,(unit['end_sample']-unit['start_sample'])/m['sample_rate'])
        result=compare_text(tokens(unit['text']),words,allow_em_initial=record['normalization_policy']['em_initial_in_scope'])
        if not result['exact']:raise ValueError('primary source mismatch')
        a,b=unit['source_span_characters'];base=m['source_span_characters'][0]
        if source[a-base:b-base]!=unit['text']:raise ValueError('source span mismatch')
        cases.append({'id':case,'expected_text':unit['text'],'offset_sample':unit['start_sample'],'source_span_characters':[a,b]})
        results.append({**result,'words':words})
        identities.append({'case':case,'model':receipt['model'],'audio_sha256':receipt['audio_sha256'],'request_sha256':rid,'response_sha256':receipt['response_sha256']})
    track,shared=candidate_track(cases,results,m['sample_rate']);groups,unresolved=highlight_groups(track)
    if track!=saved['candidate_word_track'] or groups!=saved['candidate_highlight_groups'] or shared or unresolved:raise ValueError('measured point/group identity changed')
    names=['manifest.json','source.txt','excerpt.wav','segmented-acoustic-report.json']
    names += [u['audio'] for u in m['units']]
    names += [f"transcribe/{u['request_sha256']}{suffix}" for u in saved['units'] for suffix in ['.json','.receipt.json']]
    independent=[]
    for path in sorted((root/'meta').glob('*.receipt.json')):
        receipt=load(root,str(path.relative_to(root)))
        names += [str(path.relative_to(root)),f"meta/{receipt['request_sha256']}.json"]
        if receipt['audio_sha256']!=m['audio_sha256'] or receipt['model']==saved['model']:raise ValueError('independent audio/model mismatch')
        raw=verified(root,f"meta/{receipt['request_sha256']}.json",receipt['response_sha256'])
        independent.append({'model':receipt['model'],'request_sha256':receipt['request_sha256'],
                            'response_sha256':receipt['response_sha256'],'turns':parse_turns(json.loads(raw),duration)})
    return {'schema_version':1,'provenance':'imported','provider_calls':0,'production_approved':False,
            'source_sha256':m['source_sha256'],'audio_sha256':m['audio_sha256'],'sample_rate':m['sample_rate'],
            'recipe_sha256':digest(Path(__file__).read_bytes()+Path(__file__).with_name('adapters.py').read_bytes()),
            'input_hashes':{name:digest(read(root,name)) for name in names},
            'word_track':track,'highlight_groups':groups,'primary_inputs':identities,'independent':independent,
            'reveal_permissions':[],'missing_evidence':['complete-fact-interval-verification','voice-continuity','rendered-review']}

def run(envelope):
    if set(envelope)!={'operation','root'} or envelope['operation']!='replay_acoustic':raise ValueError('unsupported helper envelope')
    return replay(Path(envelope['root']).resolve(strict=True))

if __name__=='__main__':
    try:
        raw=sys.stdin.buffer.read(65537)
        if len(raw)>65536:raise ValueError('input envelope too large')
        encoded=json.dumps(run(json.loads(raw)),ensure_ascii=False,allow_nan=False).encode()
        if len(encoded)>MAX_BYTES:raise ValueError('output envelope too large')
        sys.stdout.buffer.write(encoded+b'\n')
    except (ValueError,KeyError,TypeError,OSError) as e:
        print(json.dumps({'error':type(e).__name__,'message':'invalid offline helper input'}));sys.exit(1)
