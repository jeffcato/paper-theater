"""Bounded v2 saved chapter import. No transport, credentials or acceptance authority."""
import argparse
import base64
from decimal import Decimal
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import re
import tempfile

from chapter_acoustic import (BLOCK, CHAPTER_LIMIT, COUNT_LIMIT, FILE_LIMIT, META_LIMIT,
    TOTAL_LIMIT, encoded, digest, header, location, meta, native_header, need, read, ref,
    stream_digest)
from adapters import highlight_groups

VERSION = 2
RECIPE_FILES = ('chapter_import_helper.py', 'chapter_import.py', 'chapter_acoustic.py', 'adapters.py')
WORD = re.compile(r"[^\W_]+(?:['’][^\W_]+)*", re.UNICODE)
SCAN_LIMIT = 2_000_000


def recipe_digest():
    return digest(b''.join(Path(__file__).with_name(p).read_bytes() for p in RECIPE_FILES))


def lex(text):
    return [(''.join(c.lower() for c in m.group().replace("'", '').replace('’', '')), m.start(), m.end()) for m in WORD.finditer(text)]


def tokens(text):
    return [w[0] for w in lex(text)]


def identifier(value):
    return isinstance(value, str) and bool(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._:-]{0,127}', value))


def samples(value, rate, scale=1):
    # Preserve raw decimal spelling; no binary float subtraction or duration fabrication.
    value = str(value)
    if scale == 1:
        need(bool(re.fullmatch(r'\d+(?:\.\d{1,9})?s', value)), 'word timestamp')
        value = value[:-1]
    n = Fraction(value) * rate / scale
    need(0 <= n <= 9007199254740991, 'sample domain')
    return (n.numerator + n.denominator - 1) // n.denominator


def _refs(spec):
    result = {}
    for turn in spec['turns']:
        for role in ('primary', 'independent'):
            observation = turn.get(role)
            if observation is None:
                continue
            for key in ('request', 'response', 'receipt', 'recipe'):
                f = observation.get(key)
                if f is not None:
                    need(f['path'] not in result or result[f['path']] == f, 'conflicting raw identity')
                    result[f['path']] = f
    return result


def _raw(context):
    expected = _refs(context['spec'])
    need(set(expected) == set(context['raw']), 'raw inventory coverage')
    result = {}
    for path, f in expected.items():
        need(type(f['bytes']) == int and 0 <= f['bytes'] <= META_LIMIT, 'raw metadata limit')
        b = base64.b64decode(context['raw'][path], validate=True)
        need(len(b) == f['bytes'] and digest(b) == f['sha256'], 'raw identity')
        result[path] = b
    return result


def _observation(turn, role, raw, rate):
    obs = turn.get(role)
    result = {'outcome': 'missing', 'reason': 'observation_or_recipe_missing', 'tokens': [], 'words': [], 'start_sample': None, 'end_sample': None}
    if obs is None or any(obs.get(k) is None for k in ('request', 'response', 'receipt', 'recipe')):
        return result
    need(identifier(obs['provider']) and identifier(obs['model']), 'provider/model identity')
    receipt = json.loads(raw[obs['receipt']['path']])
    need(receipt['model'] == obs['model'] and receipt.get('provider', obs['provider']) == obs['provider']
         and receipt['request_sha256'] == obs['request']['sha256']
         and receipt['response_sha256'] == obs['response']['sha256']
         and receipt['audio_sha256'] == turn['audio']['sha256'], 'receipt/request/model/audio binding')
    if receipt.get('harness_sha256') is None or receipt.get('provider') is None:
        return result
    need(receipt['harness_sha256'] == obs['recipe']['sha256'], 'provider recipe binding')
    response = json.loads(raw[obs['response']['path']], parse_float=Decimal)
    a, b = turn['frames']
    expected = lex(turn['text'])
    if role == 'primary':
        candidates = response.get('candidates', [])
        need(len(candidates) == 1 and candidates[0].get('finishReason') == 'STOP', 'incomplete primary response')
        measured = []
        found = False
        for part in candidates[0].get('content', {}).get('parts', []):
            if 'audioTranscription' in part:
                need(isinstance(part['audioTranscription'].get('words'), list), 'word annotations missing')
                measured.extend(part['audioTranscription']['words'])
                found = True
        if not found:
            return result
        previous = [0, 0]
        for w in measured:
            x, y = samples(w['startOffset'], rate), samples(w['endOffset'], rate)
            need(0 <= x <= y <= b-a and x >= previous[0] and y >= previous[1] and len(tokens(w['word'])) == 1, 'primary interval order/domain')
            previous = [x, y]
        heard = [tokens(w['word'])[0] for w in measured]
        result['tokens'] = heard
        if heard != [w[0] for w in expected] or not heard:
            result.update(outcome='uncertain', reason='complete_primary_text_mismatch')
            return result
        result['words'] = [{'id': f'source:{turn["source_span"][0]+w[1]}:{turn["source_span"][0]+w[2]}',
            'source_span': [turn['source_span'][0]+w[1], turn['source_span'][0]+w[2]],
            'word': turn['text'][w[1]:w[2]], 'recognized_word': m['word'],
            'start_sample': a+samples(m['startOffset'], rate), 'end_sample': a+samples(m['endOffset'], rate)}
            for w, m in zip(expected, measured)]
        first, last = result['words'][0]['start_sample'], result['words'][-1]['end_sample']
    else:
        transcript, turns = response.get('transcript'), response.get('turns')
        if not isinstance(transcript, str) or not isinstance(turns, list) or not turns:
            return result
        heard, pieces, previous = tokens(transcript), [], 0
        for t in turns:
            x, y = samples(t['startMs'], rate, 1000), samples(t['endMs'], rate, 1000)
            need(0 <= x < y <= b-a and x >= previous, 'independent interval order/domain')
            previous = y
            pieces.extend(tokens(t['transcript']))
        result['tokens'] = heard
        if heard != pieces or heard != [w[0] for w in expected] or not heard:
            result.update(outcome='uncertain', reason='complete_independent_text_mismatch')
            return result
        first, last = a+samples(turns[0]['startMs'], rate, 1000), a+samples(turns[-1]['endMs'], rate, 1000)
    result.update(outcome='pass', reason='complete_saved_observation', start_sample=first, end_sample=last)
    return result


def _context(context, source):
    spec = context['spec']
    need(context['schema_version'] == VERSION and context['recipe_sha256'] == recipe_digest(), 'import context recipe/version')
    inventory = base64.b64decode(context['inventory_bytes'], validate=True)
    need(len(inventory) == context['original_inventory']['bytes'] and digest(inventory) == context['original_inventory']['sha256']
         and json.loads(inventory) == spec, 'original inventory identity')
    need(spec['schema_version'] == VERSION and spec['provenance'] in ('synthetic', 'imported', 'diagnostic') and identifier(spec['source_scope_id']), 'import schema/provenance/scope')
    need(type(spec['source_origin']) == int and 0 <= spec['source_origin'] <= 9007199254740991-len(source), 'source origin')
    need(source.encode() and digest(source.encode()) == spec['source']['sha256'] and len(source.encode()) == spec['source']['bytes'], 'complete source identity')
    frames = (spec['narration']['bytes']-44)//2
    need(spec['narration']['bytes'] == 44+2*frames and spec['narration']['bytes'] <= CHAPTER_LIMIT, 'narration domain')
    header(spec['sample_rate'], frames)
    need(1 <= len(spec['turns']) <= 1024 and len(spec['facts']) <= 1024, 'turn/fact count')
    need(len(lex(source))*max(1,sum(len(f['source_spans']) for f in spec['facts'])) <= SCAN_LIMIT, 'global ambiguity scan budget')
    raw = _raw(context)
    cursor, sample_cursor, ids, rows = spec['source_origin'], 0, set(), []
    for t in spec['turns']:
        need(identifier(t['id']) and t['id'] not in ids and isinstance(t['speaker'], str) and t['speaker'].strip(), 'turn identity/speaker')
        ids.add(t['id'])
        a, b = t['source_span']; x, y = t['frames']
        need(all(type(n) == int for n in (a, b, x, y)) and a == cursor and a < b <= spec['source_origin']+len(source)
             and source[a-spec['source_origin']:b-spec['source_origin']] == t['text'], 'verbatim ordered complete source coverage')
        need(x == sample_cursor and x < y <= frames and t['audio']['bytes'] == 44+(y-x)*2 <= FILE_LIMIT, 'ordered turn PCM coverage/limit')
        cursor, sample_cursor = b, y
        primary, independent = (_observation(t, role, raw, spec['sample_rate']) for role in ('primary', 'independent'))
        distinct = all(t.get(r) is not None for r in ('primary','independent')) and all(
            t['primary'][k] != t['independent'][k] for k in ('provider', 'model')) and all(
            t['primary'].get(k) is not None and t['independent'].get(k) is not None and t['primary'][k]['sha256'] != t['independent'][k]['sha256'] for k in ('request','response'))
        rows.append({'id':t['id'],'primary':primary,'independent':independent,'distinct':distinct})
    need(cursor == spec['source_origin']+len(source) and sample_cursor == frames, 'incomplete source/PCM tail')
    need(len(set(spec['required_fact_ids'])) == len(spec['required_fact_ids']) and set(spec['required_fact_ids']) == {f['id'] for f in spec['facts']}
         and len({f['id'] for f in spec['facts']}) == len(spec['facts']), 'fact inventory coverage')
    need(len(spec.get('inherited_findings', [])) <= 1024 and all(identifier(f['id']) and f['outcome'] in ('pass','missing','uncertain','fail') for f in spec.get('inherited_findings', [])), 'inherited finding metadata')
    return spec, rows


def _occurrences(text, needle):
    return sum(text[i:i+len(needle)] == needle for i in range(len(text)-len(needle)+1)) if needle else 0


def _facts(spec, rows, source, chunks, present):
    source_words = lex(source)
    source_tokens = [w for w,_,_ in source_words]
    primary_tokens = [w for r in rows for w in r['primary']['tokens']]
    independent_tokens = [w for r in rows for w in r['independent']['tokens']]
    complete = all(r['distinct'] and r['primary']['outcome'] == r['independent']['outcome'] == 'pass' for r in rows)
    result = []
    for f in spec['facts']:
        need(identifier(f['id']) and bool(f['source_spans']), 'fact identity/spans')
        support, ambiguous = set(), False
        for a, b in f['source_spans']:
            need(type(a) == type(b) == int and spec['source_origin'] <= a < b <= spec['source_origin']+len(source), 'fact source domain')
            text = source[a-spec['source_origin']:b-spec['source_origin']]
            phrase = tokens(text)
            full_words = [(w,x+spec['source_origin'],y+spec['source_origin']) for w,x,y in source_words]
            need(phrase and all(not (x<a<y or x<b<y) for _,x,y in full_words), 'partial source word')
            for n, stream in enumerate([source_tokens, primary_tokens, independent_tokens]):
                counts = (_occurrences(stream, phrase), _occurrences(stream, phrase[-4:]))
                ambiguous |= any(count != 1 if n == 0 or complete else count > 1 for count in counts)
            support.update(i for i,t in enumerate(spec['turns']) if t['source_span'][0] < b and a < t['source_span'][1])
        required = sorted({c['id'] for i in support for c in chunks if c['frames'][0] < spec['turns'][i]['frames'][1] and spec['turns'][i]['frames'][0] < c['frames'][1]})
        outcome, reason, bound = 'pass', 'complete_global_observations_and_pcm_dependencies', None
        if ambiguous:
            outcome, reason = 'uncertain', 'global_source_or_terminal_ambiguity'
        elif not complete:
            outcome, reason = 'missing', 'global_observations_or_independence_missing'
        elif any(abs(rows[i]['primary']['end_sample']-rows[i]['independent']['end_sample'])*4 > spec['sample_rate']*3 for i in support):
            outcome, reason = 'uncertain', 'independent_endpoint_disagreement'
        elif not set(required) <= set(present):
            outcome, reason = 'missing', 'cross_cut_evidence_missing'
        else:
            bound = max(max(rows[i]['primary']['end_sample'], rows[i]['independent']['end_sample']) for i in support)
        adverse = [previous['outcome'] for previous in spec.get('inherited_findings', [])
                   if previous['id'] in ('acoustic-fact:'+f['id'], 'chapter-import-fact:'+f['id']) and previous['outcome'] != 'pass']
        if adverse:
            rank = {'pass':0,'missing':1,'uncertain':2,'fail':3}
            outcome = max([outcome]+adverse,key=rank.get)
            reason, bound = 'recorded_adverse_fact_finding', None
        result.append({'id':f['id'],'source_spans':f['source_spans'],'turn_ids':[spec['turns'][i]['id'] for i in sorted(support)],
            'required_chunks':required,'present_chunks':[c for c in required if c in present], 'outcome':outcome,'reason':reason,'not_before_sample':bound})
    return result


def _index(root):
    index, identity = meta(root, 'chapter-import-index.json')
    need(index['schema_version'] == VERSION and index['recipe_sha256'] == recipe_digest(), 'import index recipe/version')
    context = json.loads(read(root, index['context'], META_LIMIT))
    source = read(root, index['source']).decode()
    spec, rows = _context(context, source)
    need(index['narration'] == spec['narration'] and index['source_units'] == [
        {k:t[k] for k in ('id','speaker','source_span','frames','audio')} for t in spec['turns']], 'source/narration index binding')
    cursor, ids, paths = 0, set(), set()
    need(1 <= len(index['chunks']) <= 1024, 'chunk count')
    for c in index['chunks']:
        a,b = c['frames']
        need(type(a) == type(b) == int and a == cursor and a < b and c['id'] not in ids and c['file']['path'] not in paths
             and c['file']['bytes'] == 44+(b-a)*2 <= FILE_LIMIT, 'ordered complete chunk coverage')
        ids.add(c['id']);paths.add(c['file']['path']);cursor=b
    need(cursor == (spec['narration']['bytes']-44)//2, 'incomplete chunk tail')
    return index, identity, spec, rows, source


def _check_pcm(root, index, spec, present):
    chunks = {c['id']:c for c in index['chunks']}
    checked = {}
    for key in present:
        c = chunks[key]; b = read(root, c['file'])
        need(b[:44] == header(spec['sample_rate'], c['frames'][1]-c['frames'][0]) and digest(b[44:]) == c['pcm_sha256'], 'chunk PCM identity')
        checked[key] = b
    for t in spec['turns']:
        a,b = t['frames']
        dependencies = [c for c in index['chunks'] if c['frames'][0] < b and a < c['frames'][1]]
        if all(c['id'] in checked for c in dependencies):
            h = hashlib.sha256(header(spec['sample_rate'], b-a))
            for c in dependencies:
                x,y = c['frames'];h.update(memoryview(checked[c['id']])[44+max(0,a-x)*2:44+(min(b,y)-x)*2])
            need(h.hexdigest() == t['audio']['sha256'], 'complete original turn PCM identity')
    return checked


def verify_unit(root, unit_path):
    unit, identity = meta(root, unit_path)
    index, index_ref, spec, rows, source = _index(root)
    need(unit['schema_version'] == VERSION and unit['recipe_sha256'] == recipe_digest() and unit['index'] == index_ref, 'unit version/index')
    present = unit['included_chunks'];chunks = {c['id']:c for c in index['chunks']}
    need(len(present) == len(set(present)) and unit['base_chunk'] in present and set(present) <= set(chunks), 'unit chunk selection')
    expected = sorted([index['source'], index['context'], index_ref]+[chunks[k]['file'] for k in present], key=lambda f:f['path'])
    need(unit['files'] == expected and len(expected)+5 <= COUNT_LIMIT and sum(f['bytes'] for f in expected)+5*META_LIMIT <= TOTAL_LIMIT, 'unit reference/aggregate limits')
    _check_pcm(root, index, spec, present)
    a,b = chunks[unit['base_chunk']]['frames']
    words = [w for r in rows for w in r['primary']['words']]
    groups, unresolved = highlight_groups(words)
    def clip(items):
        return [{'original':w,'global_samples':[w['start_sample'],w['end_sample']],
                 'local_samples':[max(a,w['start_sample'])-a,min(b,w['end_sample'])-a],
                 'complete':a<=w['start_sample'] and w['end_sample']<=b} for w in items
                if (w['start_sample']==w['end_sample'] and a<=w['start_sample']<b) or (w['start_sample']<b and a<w['end_sample'])]
    report = {'schema_version':VERSION,'provenance':'diagnostic','observation_provenance':spec['provenance'],
        'production_approved':False,'provider_calls':0,'outcome':'needs_repair','recipe_sha256':recipe_digest(),
        'index_sha256':index_ref['sha256'],'context_sha256':index['context']['sha256'],'unit_sha256':identity['sha256'],
        'source_sha256':index['source']['sha256'],'chapter_sha256':spec['narration']['sha256'],'base_chunk':unit['base_chunk'],
        'global_frames':[a,b],'checked_files':{f['path']:f['sha256'] for f in expected},
        'complete_source_coverage':True,'global_observation_coverage':all(r['distinct'] and r['primary']['outcome']==r['independent']['outcome']=='pass' for r in rows),
        'source_scalar_count':len(source),'word_count':len(words),'highlight_group_count':len(groups),
        'point_word_count':sum(w['start_sample']==w['end_sample'] for w in words),'unresolved_point_ids':unresolved,
        'words':clip(words),'groups':clip(groups),'facts':_facts(spec,rows,source,index['chunks'],present),
        'reveal_permissions':[],'acceptance_policy_authority':False}
    need(len(encoded(report)) <= META_LIMIT, 'helper output limit')
    return report


def verify_bundle(root):
    index, identity, spec, rows, source = _index(root)
    h = hashlib.sha256(header(spec['sample_rate'], (spec['narration']['bytes']-44)//2))
    for c in index['chunks']:
        b = read(root, c['file']);need(digest(b[44:]) == c['pcm_sha256'], 'chunk PCM identity');h.update(memoryview(b)[44:])
        verify_unit(root, f"units/{c['id']}.json")
    need(h.hexdigest() == spec['narration']['sha256'], 'complete chapter PCM identity')
    # Stream turn verification separately; no full chapter allocation.
    for t in spec['turns']:
        needed = [c['id'] for c in index['chunks'] if c['frames'][0] < t['frames'][1] and t['frames'][0] < c['frames'][1]]
        _check_pcm(root,index,spec,needed)
    return {'schema_version':VERSION,'provenance':'diagnostic','production_approved':False,'provider_calls':0,
        'outcome':'needs_repair','recipe_sha256':recipe_digest(),'index_sha256':identity['sha256'],
        'source_scalar_count':len(source),'complete_source_coverage':True,'complete_pcm_coverage':True,
        'chapter_sha256':h.hexdigest(),'facts':_facts(spec,rows,source,index['chunks'],[c['id'] for c in index['chunks']]),'reveal_permissions':[]}


def produce(root, evidence_path, output, *, max_frames=None):
    output = Path(output)
    need(not output.exists(), 'output already exists')
    spec, evidence_ref = meta(root, evidence_path)
    # Reserve all metadata/raw/source/turn envelopes before reading evidence bytes.
    need(spec['schema_version'] == VERSION and 1 <= len(spec['turns']) <= 1024, 'import version/turn count')
    refs = _refs(spec)
    need(spec['source']['bytes'] <= FILE_LIMIT and spec['narration']['bytes'] <= CHAPTER_LIMIT
         and all(0 <= f['bytes'] <= META_LIMIT for f in refs.values())
         and sum(f['bytes'] for f in refs.values()) <= META_LIMIT//2
         and all(44 < t['audio']['bytes'] <= FILE_LIMIT for t in spec['turns']), 'import evidence allocation limit')
    raw = {p:base64.b64encode(read(root,f,META_LIMIT)).decode('ascii') for p,f in refs.items()}
    inventory_bytes = read(root,evidence_ref,META_LIMIT)
    context = {'schema_version':VERSION,'recipe_sha256':recipe_digest(),'original_inventory':evidence_ref,
               'inventory_bytes':base64.b64encode(inventory_bytes).decode('ascii'),'spec':spec,'raw':raw}
    context_bytes = encoded(context);need(len(context_bytes) <= META_LIMIT, 'bounded global context limit')
    source_bytes = read(root,spec['source']);source = source_bytes.decode()
    spec, rows = _context(context,source)
    narration = location(root,spec['narration']['path']);need(narration.stat().st_size == spec['narration']['bytes'], 'chapter length')
    with narration.open('rb') as stream:rate,frames = native_header(stream.read(44),narration.stat().st_size)
    need(rate == spec['sample_rate'] and stream_digest(narration) == spec['narration']['sha256'], 'chapter WAV identity')
    with narration.open('rb') as stream:
        for t in spec['turns']:
            b = read(root,t['audio']);a,z=t['frames'];stream.seek(44+a*2)
            need(b[:44] == header(rate,z-a) and b[44:] == stream.read((z-a)*2), 'original turn PCM slice')
    capacity = min((FILE_LIMIT-44)//2, (TOTAL_LIMIT-len(context_bytes)-len(source_bytes)-6*META_LIMIT-44)//2)
    if max_frames is None:max_frames=capacity
    need(type(max_frames)==int and 0<max_frames<=capacity and (frames+max_frames-1)//max_frames<=1024, 'chapter chunk capacity')
    output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chapter-import-',dir=output.parent) as temp:
        target = Path(temp)
        def save(path,body,media='application/json'):
            p=location(target,path);p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body);return ref(path,body,media)
        source_ref = save('private/source.txt',source_bytes,'text/plain')
        context_ref = save('private/chapter-context.json',context_bytes)
        index={'schema_version':VERSION,'provenance':'diagnostic','recipe_sha256':recipe_digest(),'source':source_ref,
            'context':context_ref,'narration':spec['narration'],'source_units':[{k:t[k] for k in ('id','speaker','source_span','frames','audio')} for t in spec['turns']],
            'limits':{'file':FILE_LIMIT,'metadata':META_LIMIT,'aggregate':TOTAL_LIMIT,'references':COUNT_LIMIT,'output':META_LIMIT,'replay_metadata_slots':5,'global_context_bytes':len(context_bytes)},'chunks':[]}
        with narration.open('rb') as stream:
            stream.seek(44)
            for n,a in enumerate(range(0,frames,max_frames)):
                z=min(a+max_frames,frames);body=header(rate,z-a)+stream.read((z-a)*2);key=f'chunk-{n:04}'
                f=save(f'private/chunks/{key}.wav',body,'audio/wav')
                index['chunks'].append({'id':key,'frames':[a,z],'file':f,'pcm_sha256':digest(body[44:])})
        index_bytes=encoded(index);need(len(index_bytes)<=META_LIMIT,'index metadata limit')
        index_ref=save('chapter-import-index.json',index_bytes)
        for base in index['chunks']:
            present=[base['id']]
            # Keep all requirements explicit; dependencies that cannot fit stay unresolved.
            for c in index['chunks']:
                if c['id'] in present:continue
                wanted = [index['source'],index['context'],index_ref]+[x['file'] for x in index['chunks'] if x['id'] in present+[c['id']]]
                if len(wanted)+5<=COUNT_LIMIT and sum(f['bytes'] for f in wanted)+5*META_LIMIT<=TOTAL_LIMIT:present.append(c['id'])
            present.sort()
            files=sorted([source_ref,context_ref,index_ref]+[c['file'] for c in index['chunks'] if c['id'] in present],key=lambda f:f['path'])
            unit={'schema_version':VERSION,'recipe_sha256':recipe_digest(),'index':index_ref,'base_chunk':base['id'],'included_chunks':present,'files':files}
            path=f"units/{base['id']}.json";body=encoded(unit);need(len(body)<=META_LIMIT,'unit metadata limit');save(path,body)
            report=verify_unit(target,path);save(f"records/{base['id']}.json",encoded(report))
        report=verify_bundle(target);save('private/chapter-import-verification.json',encoded(report))
        # Preserve exact original raw bytes in a private audit directory, independently of context encoding.
        for p,f in refs.items():save('private/originals/'+p,base64.b64decode(raw[p]),f['media_type'])
        save('private/originals/'+evidence_path,inventory_bytes)
        target.rename(output)
    return index


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root');parser.add_argument('evidence');parser.add_argument('output');parser.add_argument('--max-frames',type=int)
    args=parser.parse_args();produce(Path(args.root),args.evidence,Path(args.output),max_frames=args.max_frames)
