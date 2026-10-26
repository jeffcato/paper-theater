import hashlib,json,unittest

def proof(response,model,source,offset=0):
    raw=json.dumps(response).encode()
    return dict(raw=raw,response_sha256=hashlib.sha256(raw).hexdigest(),
        source_sha256=hashlib.sha256(source.encode()).hexdigest(),audio_sha256='a'*64,
        request_sha256='b'*64,recipe_sha256='c'*64,model=model,offset_sample=offset)

def fixture():
    source='A storm came. Dorothy held Toto.'
    primary=proof({'candidates':[{'finishReason':'STOP','content':{'parts':[{'audioTranscription':{'words':[
        {'word':w,'startOffset':f'{i+1}s','endOffset':f'{i+2}s'} for i,w in enumerate(['Dorothy','held','Toto'])]}}]}}]},'primary',source)
    independent=proof({'transcript':source,'turns':[
        {'startMs':0,'endMs':900,'transcript':'A storm came.'},
        {'startMs':1000,'endMs':4250,'transcript':'Dorothy held Toto.'}]},'independent',source)
    return source,[14,len(source)],[{'id':'holding','source_span':[14,len(source)]}],primary,independent

class EvidenceTests(unittest.TestCase):
    def test_complete_unique_turn_supplies_later_bound_for_every_fact(self):
        from evidence import match_facts
        result=match_facts(*fixture(),sample_rate=24000,sample_count=240000)
        self.assertEqual(result['facts'][0]['not_before_sample'],102000)
        self.assertEqual(result['facts'][0]['fact_id'],'holding')
        self.assertFalse(result['production_approved'])

    def test_nearby_terminal_does_not_override_missing_or_ambiguous_evidence(self):
        from evidence import match_facts
        def replace_raw(record,response):
            record['raw']=json.dumps(response).encode()
            record['response_sha256']=hashlib.sha256(record['raw']).hexdigest()
        for defect in ['source_hash','audio_hash','raw_hash','recipe_hash','same_model','missing_fact',
                       'outside_tolerance','unordered_turns','repeated_terminal','repeated_independent','incomplete','point_terminal']:
            with self.subTest(defect=defect):
                source,span,facts,p,q=fixture()
                if defect=='source_hash':q['source_sha256']='0'*64
                if defect=='audio_hash':q['audio_sha256']='0'*64
                if defect=='raw_hash':q['response_sha256']='0'*64
                if defect=='recipe_hash':q['recipe_sha256']='unknown'
                if defect=='same_model':q['model']=p['model']
                if defect=='missing_fact':facts.append({'id':'storm','source_span':[0,13]})
                if defect=='outside_tolerance':
                    r=json.loads(q['raw']);r['turns'][1]['endMs']=4751;replace_raw(q,r)
                if defect=='unordered_turns':
                    r=json.loads(q['raw']);r['turns'].reverse();replace_raw(q,r)
                if defect=='repeated_terminal':
                    source+=' Dorothy held Toto.'
                    for r in [p,q]:r['source_sha256']=hashlib.sha256(source.encode()).hexdigest()
                if defect=='repeated_independent':
                    r=json.loads(q['raw']);r['transcript']+=' Dorothy held Toto.';r['turns'].append(dict(r['turns'][1]));replace_raw(q,r)
                if defect=='incomplete':
                    r=json.loads(q['raw']);r['turns'][1]['transcript']='held Toto.';replace_raw(q,r)
                if defect=='point_terminal':
                    r=json.loads(p['raw']);r['candidates'][0]['content']['parts'][0]['audioTranscription']['words'][-1]['endOffset']='3s';replace_raw(p,r)
                with self.assertRaises(ValueError):match_facts(source,span,facts,p,q,sample_rate=24000,sample_count=240000)
