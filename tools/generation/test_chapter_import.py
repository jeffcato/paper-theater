"""Synthetic saved observations only; no provider responses are purchased or claimed."""
import json
from pathlib import Path
import re
import socket
import struct
import tempfile
import unittest
from unittest.mock import patch

import chapter_acoustic as old

ROOT = Path(__file__).resolve().parents[2]


def fixture(root, texts=None):
    def put(path, body, media='application/json'):
        p = root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(body)
        return old.ref(path, body, media)

    texts = texts or ['Élodie whispered, “Open the gate.”\n', '李 answered, “The moon is blue.”\n', 'Then the silver bell rang.']
    source = ''.join(texts)
    rate, size = 8000, 8000
    pcm = b''.join(struct.pack('<h', (n % 301) - 150) for n in range(size * len(texts)))
    spec = {'schema_version': 2, 'provenance': 'synthetic', 'source_scope_id': 'synthetic-chapter-v2',
            'source_origin': 700, 'sample_rate': rate,
            'source': put('source.txt', source.encode(), 'text/plain'),
            'narration': put('chapter.wav', old.header(rate, size * len(texts)) + pcm, 'audio/wav'),
            'turns': [], 'facts': [], 'required_fact_ids': ['gate', 'cross-turn']}
    cursor = 700
    for n, text in enumerate(texts):
        audio = put(f'turns/speech-{n}.wav', old.header(rate, size) + pcm[n*size*2:(n+1)*size*2], 'audio/wav')
        turn = {'id': f'speech-{n}', 'speaker': ['Élodie', '李', 'Narrator'][n % 3], 'text': text,
                'source_span': [cursor, cursor + len(text)], 'frames': [n*size, (n+1)*size], 'audio': audio}
        cursor += len(text)
        words = re.findall(r"[^\W_]+(?:['’][^\W_]+)*", text)
        primary = {'candidates': [{'finishReason': 'STOP', 'content': {'parts': [{'audioTranscription': {'words': [
            {'word': w, 'startOffset': f'{i/len(words):.9f}s', 'endOffset': f'{(i+1)/len(words):.9f}s'}
            for i, w in enumerate(words)]}}]}}]}
        independent = {'transcript': text, 'turns': [{'startMs': 0, 'endMs': 1000, 'transcript': text}]}
        for role, response in [('primary', primary), ('independent', independent)]:
            provider, model = f'synthetic-{role}', f'synthetic-{role}-v2'
            recipe = put(f'raw/{n}-{role}.recipe', f'synthetic {role} recipe v2'.encode(), 'text/plain')
            request = put(f'raw/{n}-{role}.request.json', old.encoded({'provider': provider, 'model': model, 'audio_sha256': audio['sha256'], 'synthetic': True}))
            response = put(f'raw/{n}-{role}.response.json', old.encoded(response))
            receipt = put(f'raw/{n}-{role}.receipt.json', old.encoded({'provider': provider, 'model': model,
                'request_sha256': request['sha256'], 'response_sha256': response['sha256'],
                'audio_sha256': audio['sha256'], 'harness_sha256': recipe['sha256'], 'provenance': 'synthetic'}))
            turn[role] = {'provider': provider, 'model': model, 'request': request, 'response': response, 'receipt': receipt, 'recipe': recipe}
        spec['turns'].append(turn)
    gate = source.index('Open') + 700
    spec['facts'] = [{'id': 'gate', 'source_spans': [[gate, gate + len('Open the gate')]]},
                     {'id': 'cross-turn', 'source_spans': [[700, 700 + len(texts[0]) + len(texts[1])]]}]
    put('chapter-evidence.json', old.encoded(spec))
    return spec


def rewrite_response(root, obs, body):
    (root/obs['response']['path']).write_bytes(body)
    obs['response']=old.ref(obs['response']['path'],body)
    receipt=json.loads((root/obs['receipt']['path']).read_bytes())
    receipt['response_sha256']=obs['response']['sha256'];body=old.encoded(receipt)
    (root/obs['receipt']['path']).write_bytes(body);obs['receipt']=old.ref(obs['receipt']['path'],body)


def build(root, variant=''):
    """Reusable Rust/Python synthetic fixture producer; large WAVs remain temporary."""
    import chapter_import as c
    repeated=['Élodie whispered, “Open the gate.”\n']*2+['A different final bell rang.']
    spec=fixture(root,repeated if variant=='ambiguous' else None)
    if variant=='missing':spec['turns'][1]['independent']=None
    if variant=='adverse':spec['inherited_findings']=[{'id':'acoustic-fact:gate','field':'private.original','outcome':'fail','message':'retained original failure','evidence':[]}]
    if variant=='extra':
        text=spec['turns'][2]['text']+' Open the gate.'
        rewrite_response(root,spec['turns'][2]['independent'],old.encoded({'transcript':text,'turns':[{'startMs':0,'endMs':1000,'transcript':text}]}))
    if variant in ('tolerance-equal','tolerance-outside'):
        text=spec['turns'][0]['text'];end=250 if variant=='tolerance-equal' else 249.875
        rewrite_response(root,spec['turns'][0]['independent'],old.encoded({'transcript':text,'turns':[{'startMs':0,'endMs':end,'transcript':text}]}))
    if variant=='precision':
        obs=spec['turns'][0]['primary'];value=json.loads((root/obs['response']['path']).read_bytes());words=value['candidates'][0]['content']['parts'][0]['audioTranscription']['words']
        for i,w in enumerate(words):w.update(startOffset=f'{i/len(words)/4:.9f}s',endOffset=f'{(i+1)/len(words)/4:.9f}s')
        rewrite_response(root,obs,old.encoded(value))
        obs=spec['turns'][0]['independent'];body=old.encoded({'transcript':spec['turns'][0]['text'],'turns':[{'startMs':0,'endMs':250,'transcript':spec['turns'][0]['text']}]})
        rewrite_response(root,obs,body.replace(b'"endMs":250',b'"endMs":250.0000000000000001'))
    if variant=='large':
        frames=3_200_000;rate=spec['sample_rate'];original=[(root/t['audio']['path']).read_bytes()[44:] for t in spec['turns']]
        with (root/'chapter.wav').open('wb') as f:
            f.write(old.header(rate,frames*3));f.truncate(44+frames*6)
            for i,pcm in enumerate(original):f.seek(44+i*frames*2);f.write(pcm)
        spec['narration']={**spec['narration'],'bytes':44+frames*6,'sha256':old.stream_digest(root/'chapter.wav')}
        for i,t in enumerate(spec['turns']):
            path=root/t['audio']['path']
            with path.open('wb') as f:f.write(old.header(rate,frames));f.write(original[i]);f.truncate(44+frames*2)
            t['frames']=[i*frames,(i+1)*frames];t['audio']={**t['audio'],'bytes':44+frames*2,'sha256':old.stream_digest(path)}
            for role in ('primary','independent'):
                obs=t[role];value=json.loads((root/obs['request']['path']).read_bytes());value['audio_sha256']=t['audio']['sha256'];body=old.encoded(value)
                (root/obs['request']['path']).write_bytes(body);obs['request']=old.ref(obs['request']['path'],body)
                receipt=json.loads((root/obs['receipt']['path']).read_bytes());receipt.update(audio_sha256=t['audio']['sha256'],request_sha256=obs['request']['sha256']);body=old.encoded(receipt)
                (root/obs['receipt']['path']).write_bytes(body);obs['receipt']=old.ref(obs['receipt']['path'],body)
    (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
    index=c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=None if variant=='large' else 400 if variant=='many' else 6000)
    if variant=='cut':
        path=root/'bundle/units/chunk-0000.json';unit=json.loads(path.read_bytes());unit['included_chunks'].remove('chunk-0001')
        missing=index['chunks'][1]['file']['path'];unit['files']=[f for f in unit['files'] if f['path']!=missing];path.write_bytes(old.encoded(unit))
        (root/'bundle/records/chunk-0000.json').write_bytes(old.encoded(c.verify_unit(root/'bundle','units/chunk-0000.json')))
    return index


class ChapterImportTests(unittest.TestCase):
    def test_distinct_complete_source_imports_with_unicode_turns_and_global_dependencies(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(socket, 'socket', side_effect=AssertionError('provider call')):
            root = Path(temp)
            spec = fixture(root)
            with self.assertRaisesRegex(ValueError, 'source identity mismatch'):
                old.produce(root/'chapter.wav', root/'source.txt', ROOT/'fixtures/generation/presentation-evidence-v1/input-index.json', root/'old')
            from chapter_import import produce, verify_bundle, verify_unit
            index = produce(root, 'chapter-evidence.json', root/'bundle', max_frames=6000)
            report = verify_bundle(root/'bundle')
            self.assertEqual(report['source_scalar_count'], len(''.join(t['text'] for t in spec['turns'])))
            self.assertTrue(report['complete_source_coverage'])
            self.assertTrue(report['complete_pcm_coverage'])
            self.assertEqual(report['outcome'], 'needs_repair')
            self.assertEqual(report['provider_calls'], 0)
            unit = verify_unit(root/'bundle', 'units/chunk-0000.json')
            gate = next(f for f in unit['facts'] if f['id'] == 'gate')
            self.assertEqual(gate['required_chunks'], ['chunk-0000', 'chunk-0001'])
            self.assertEqual(gate['not_before_sample'], 8000)
            self.assertEqual(gate['outcome'], 'pass')
            self.assertEqual(index['source_units'][1]['speaker'], '李')
            self.assertEqual(verify_unit(root/'bundle', 'units/chunk-0000.json'), unit)

    def test_missing_transcript_recipe_or_independence_keeps_global_bounds_unknown(self):
        import chapter_import as c
        for fault in ['independent', 'primary', 'recipe', 'same-model', 'empty-transcript', 'provider']:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temp:
                root=Path(temp);spec=fixture(root)
                if fault in ('independent','primary'):spec['turns'][1][fault]=None
                if fault=='recipe':spec['turns'][1]['independent']['recipe']=None
                if fault=='same-model':spec['turns'][1]['independent']=spec['turns'][1]['primary']
                if fault=='provider':
                    obs=spec['turns'][1]['independent'];receipt=json.loads((root/obs['receipt']['path']).read_bytes());receipt.pop('provider')
                    body=old.encoded(receipt);(root/obs['receipt']['path']).write_bytes(body);obs['receipt']=old.ref(obs['receipt']['path'],body)
                if fault=='empty-transcript':
                    obs=spec['turns'][1]['independent'];body=old.encoded({'transcript':'','turns':[]})
                    (root/obs['response']['path']).write_bytes(body);obs['response']=old.ref(obs['response']['path'],body)
                    receipt=json.loads((root/obs['receipt']['path']).read_bytes());receipt['response_sha256']=obs['response']['sha256'];body=old.encoded(receipt)
                    (root/obs['receipt']['path']).write_bytes(body);obs['receipt']=old.ref(obs['receipt']['path'],body)
                (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
                c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=6000)
                report=c.verify_bundle(root/'bundle')
                self.assertTrue(report['complete_source_coverage'])
                self.assertTrue(all(f['not_before_sample'] is None for f in report['facts']))
                self.assertTrue(all(f['outcome'] != 'pass' for f in report['facts']))

    def test_missing_cross_cut_pcm_and_repeated_global_text_never_become_local_matches(self):
        import chapter_import as c
        for repeated in [False,True]:
            with self.subTest(repeated=repeated),tempfile.TemporaryDirectory() as temp:
                root=Path(temp)
                fixture(root, ['Élodie whispered, “Open the gate.”\n']*2+['A different final bell rang.'] if repeated else None)
                index=c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=6000)
                path=root/'bundle/units/chunk-0000.json';unit=json.loads(path.read_bytes())
                missing='chunk-0001';unit['included_chunks'].remove(missing)
                missing_path=next(x['file']['path'] for x in index['chunks'] if x['id']==missing)
                unit['files']=[f for f in unit['files'] if f['path']!=missing_path];path.write_bytes(old.encoded(unit))
                report=c.verify_unit(root/'bundle','units/chunk-0000.json')
                gate=next(f for f in report['facts'] if f['id']=='gate')
                self.assertIsNone(gate['not_before_sample'])
                self.assertEqual(gate['outcome'],'uncertain' if repeated else 'missing')
                self.assertEqual(gate['reason'],'global_source_or_terminal_ambiguity' if repeated else 'cross_cut_evidence_missing')

    def test_changed_bindings_missing_duplicate_reordered_turns_and_pcm_are_rejected(self):
        import chapter_import as c
        for fault in ['missing-turn','duplicate-turn','reorder','source-span','speaker','model','request','response','receipt','recipe','audio','source','narration']:
            with self.subTest(fault=fault),tempfile.TemporaryDirectory() as temp:
                root=Path(temp);spec=fixture(root)
                if fault=='missing-turn':spec['turns'].pop(1)
                elif fault=='duplicate-turn':spec['turns'].insert(0,spec['turns'][0])
                elif fault=='reorder':spec['turns'].reverse()
                elif fault=='source-span':spec['turns'][1]['source_span'][0]-=1
                elif fault=='speaker':spec['turns'][1]['speaker']=''
                elif fault=='model':spec['turns'][0]['primary']['model']='unprovided-model'
                else:
                    f=spec['turns'][0]['primary'][fault] if fault in ['request','response','receipt','recipe'] else spec['turns'][0]['audio'] if fault=='audio' else spec[fault]
                    path=root/f['path'];body=bytearray(path.read_bytes());body[-1]^=1;path.write_bytes(body)
                (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
                with self.assertRaises((ValueError,KeyError)):c.produce(root,'chapter-evidence.json',root/'bundle')
                self.assertFalse((root/'bundle').exists())

    def test_oversized_declared_evidence_fails_before_any_payload_read(self):
        import chapter_import as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);spec=fixture(root);spec['turns'][0]['primary']['response']['bytes']=old.META_LIMIT+1
            (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
            with patch.object(c,'read',side_effect=AssertionError('payload read before limit rejection')):
                with self.assertRaisesRegex(ValueError,'allocation limit'):c.produce(root,'chapter-evidence.json',root/'bundle')

    def test_saved_helper_and_raw_receipt_bytes_replay_exactly_without_network(self):
        import chapter_import as c
        import subprocess
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);spec=fixture(root);c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=6000)
            for f in c._refs(spec).values():self.assertEqual((root/f['path']).read_bytes(),(root/'bundle/private/originals'/f['path']).read_bytes())
            request=old.encoded({'operation':'verify_imported_chapter_unit_v2','root':str(root/'bundle'),'unit':'units/chunk-0000.json'})
            result=subprocess.run(['/usr/bin/python3','-E','-s','-S',str(ROOT/'tools/generation/chapter_import_helper.py')],input=request,capture_output=True,timeout=30,check=True)
            self.assertEqual(result.stdout,(root/'bundle/records/chunk-0000.json').read_bytes())
            self.assertNotEqual(c.recipe_digest(),old.recipe_digest())

    def test_inherited_adverse_fact_findings_survive_new_matching_observations(self):
        import chapter_import as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);spec=fixture(root)
            spec['inherited_findings']=[{'id':'acoustic-fact:gate','field':'private.original','outcome':'fail','message':'retained original failure','evidence':[]}]
            (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
            c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=6000)
            gate=c.verify_unit(root/'bundle','units/chunk-0000.json')['facts'][0]
            self.assertEqual(gate['outcome'],'fail')
            self.assertIsNone(gate['not_before_sample'])

    def test_exact_decimal_ceiling_and_temporary_large_generic_master(self):
        import chapter_import as c
        for variant in ['precision','large']:
            with self.subTest(variant=variant),tempfile.TemporaryDirectory() as temp:
                root=Path(temp);index=build(root,variant)
                report=c.verify_unit(root/'bundle','units/chunk-0000.json')
                if variant=='precision':self.assertEqual(report['facts'][0]['not_before_sample'],2001)
                else:
                    self.assertGreater((root/'chapter.wav').stat().st_size,c.FILE_LIMIT)
                    self.assertEqual(index['chunks'][0]['file']['bytes'],c.FILE_LIMIT)
                    self.assertTrue(c.verify_bundle(root/'bundle')['complete_pcm_coverage'])

    def test_rebound_chunk_hashes_cannot_hide_changed_original_turn_pcm(self):
        import chapter_import as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);index=build(root)
            chunk=index['chunks'][0];path=root/'bundle'/chunk['file']['path'];b=bytearray(path.read_bytes());b[-1]^=1;path.write_bytes(b)
            chunk['file']=old.ref(chunk['file']['path'],b,'audio/wav');chunk['pcm_sha256']=old.digest(b[44:])
            path=root/'bundle/chapter-import-index.json';body=old.encoded(index);path.write_bytes(body)
            path=root/'bundle/units/chunk-0000.json';unit=json.loads(path.read_bytes());unit['index']=old.ref('chapter-import-index.json',body)
            unit['files']=[unit['index'] if f['path']=='chapter-import-index.json' else chunk['file'] if f['path']==chunk['file']['path'] else f for f in unit['files']];path.write_bytes(old.encoded(unit))
            with self.assertRaisesRegex(ValueError,'original turn PCM'):c.verify_unit(root/'bundle','units/chunk-0000.json')

    def test_unsupported_global_ambiguity_work_is_rejected_without_an_output(self):
        import chapter_import as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);words=' '.join('word'+str(i) for i in range(1000))
            spec=fixture(root,['Élodie whispered, “Open the gate.” '+words+'\n', '李 answered. '+words+'\n', 'The silver bell rang. '+words])
            fact=spec['facts'][0];spec['facts']=[{**fact,'id':f'fact-{i}'} for i in range(1024)];spec['required_fact_ids']=[f['id'] for f in spec['facts']]
            (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
            with self.assertRaisesRegex(ValueError,'global ambiguity scan budget'):c.produce(root,'chapter-evidence.json',root/'bundle')
            self.assertFalse((root/'bundle').exists())

    def test_extra_independent_text_preserves_ambiguity_across_the_complete_chapter(self):
        import chapter_import as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);spec=fixture(root);obs=spec['turns'][2]['independent']
            text=spec['turns'][2]['text']+' Open the gate.'
            rewrite_response(root,obs,old.encoded({'transcript':text,'turns':[{'startMs':0,'endMs':1000,'transcript':text}]}))
            (root/'chapter-evidence.json').write_bytes(old.encoded(spec))
            c.produce(root,'chapter-evidence.json',root/'bundle',max_frames=6000)
            gate=c.verify_unit(root/'bundle','units/chunk-0000.json')['facts'][0]
            self.assertEqual(gate['outcome'],'uncertain')
            self.assertIsNone(gate['not_before_sample'])

    def test_exact_three_quarter_second_rule_uses_the_later_endpoint(self):
        import chapter_import as c
        for variant,expected in [('tolerance-equal',8000),('tolerance-outside',None)]:
            with self.subTest(variant=variant),tempfile.TemporaryDirectory() as temp:
                root=Path(temp);build(root,variant)
                row=c.verify_unit(root/'bundle','units/chunk-0000.json')['facts'][0]
                self.assertEqual(row['not_before_sample'],expected)
                self.assertEqual(row['outcome'],'pass' if expected is not None else 'uncertain')


if __name__ == '__main__':
    unittest.main()
