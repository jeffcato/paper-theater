"""Conservative fact observations from independently measured complete turns."""
import hashlib,json,re
from adapters import parse_words,parse_turns,tokens,sample_at,word_check


def match_facts(source,turn_span,facts,primary,independent,*,sample_rate,sample_count):
    if type(sample_rate)!=int or sample_rate<=0 or type(sample_count)!=int or sample_count<=0:raise ValueError('invalid audio domain')
    for record in [primary,independent]:
        for key in ['source_sha256','audio_sha256','request_sha256','recipe_sha256','response_sha256']:
            if not re.fullmatch('[0-9a-f]{64}',record[key]):raise ValueError('invalid evidence identity')
        if hashlib.sha256(record['raw']).hexdigest()!=record['response_sha256']:raise ValueError('raw response hash mismatch')
        if record['source_sha256']!=hashlib.sha256(source.encode()).hexdigest():raise ValueError('source hash mismatch')
        if type(record['offset_sample'])!=int or not 0<=record['offset_sample']<sample_count:raise ValueError('invalid placement offset')
    if primary['audio_sha256']!=independent['audio_sha256'] or primary['model']==independent['model']:raise ValueError('audio mismatch or non-independent model')
    a,b=turn_span
    if not 0<=a<b<=len(source) or not facts:raise ValueError('missing source/fact scope')
    for fact in facts:
        c,d=fact['source_span']
        if not a<=c<d<=b:raise ValueError('fact not completely covered by interval')
    if len({f['id'] for f in facts})!=len(facts):raise ValueError('duplicate fact ID')
    expected=tokens(source[a:b])
    words=parse_words(json.loads(primary['raw']))
    turns=parse_turns(json.loads(independent['raw']),sample_count/sample_rate)
    matched=[t for t in turns if tokens(t['text'])==expected]
    if not expected or not words or len(matched)!=1:raise ValueError('incomplete or ambiguous independent interval')
    word_check(words,(sample_count-primary['offset_sample'])/sample_rate)
    if [t for w in words for t in tokens(w['word'])]!=expected:raise ValueError('primary source mismatch')
    terminal=expected[-min(3,len(expected)):]
    full=tokens(json.loads(independent['raw'])['transcript'])
    def count(sequence):return sum(sequence[i:i+len(terminal)]==terminal for i in range(len(sequence)-len(terminal)+1))
    if len(terminal)<2 or count(tokens(source))!=1 or count(full)!=1:raise ValueError('ambiguous terminal')
    if words[-1]['start']>=words[-1]['end']:raise ValueError('terminal endpoint is a point')
    p=primary['offset_sample']+sample_at(words[-1]['end'],sample_rate)
    q=independent['offset_sample']+sample_at(matched[0]['end'],sample_rate)
    if max(p,q)>sample_count or abs(p-q)*4>sample_rate*3:raise ValueError('endpoint disagreement exceeds 0.75 seconds')
    return {'production_approved':False,'facts':[{'fact_id':f['id'],'source_span':f['source_span'],
        'primary_anchor_id':primary['response_sha256'],'independent_anchor_id':independent['response_sha256'],
        'not_before_sample':max(p,q)} for f in facts]}
