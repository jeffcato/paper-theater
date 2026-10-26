import hashlib,io,json,socket,unittest,wave
from pathlib import Path
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'fixtures/shared/two-scene-v1'

class ReadinessCorrectionTests(unittest.TestCase):
    def test_new_revision_keeps_measured_source_and_blackout_contracts_while_fixing_readiness(self):
        from readiness_correction import prepare_revision
        baseline=json.loads((BASE/'fixture.json').read_text())
        old_hashes={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in BASE.rglob('*') if p.is_file()}
        with patch.object(socket,'socket',side_effect=AssertionError('provider dispatch')):
            revision=prepare_revision(ROOT)
        doc=revision['document'];timeline=doc['timeline'];tr=timeline['transitions'][0]
        self.assertNotEqual(revision['record']['manifest_sha256'],hashlib.sha256((BASE/'manifest.json').read_bytes()).hexdigest())
        self.assertEqual(revision['record']['revision_id'],'two-scene-readiness-v2')
        self.assertFalse(revision['record']['production_approved'])
        self.assertEqual(revision['record']['provider_calls'],0)
        self.assertLessEqual(doc['manifest']['reveals'][1]['not_before_sample'],tr['fade_in_start'])
        self.assertEqual(tr['swap_sample']-tr['fade_out_start'],48000)
        self.assertEqual(tr['fade_in_start']-tr['swap_sample'],96000)
        self.assertEqual(tr['fade_in_end']-tr['fade_in_start'],48000)
        self.assertEqual(timeline['scenes'][0]['narration'][1],tr['fade_out_start'])
        self.assertEqual(timeline['scenes'][1]['narration'][0],tr['fade_in_end'])
        self.assertEqual(doc['source_text'],baseline['source_text'])
        self.assertEqual(timeline['sample_count'],baseline['timeline']['sample_count'])
        self.assertEqual(timeline['closing'],baseline['timeline']['closing'])
        self.assertEqual(doc['read_along']['alignment_sha256'],baseline['read_along']['alignment_sha256'])
        self.assertEqual(doc['read_along']['narration_sha256'],baseline['read_along']['narration_sha256'])
        self.assertEqual([w['id'] for w in doc['read_along']['words']],[w['id'] for w in baseline['read_along']['words']])
        self.assertEqual([g['word_ids'] for g in doc['read_along']['groups']],[g['word_ids'] for g in baseline['read_along']['groups']])
        self.assertEqual(sum(w['start_sample']==w['end_sample'] for w in doc['read_along']['words']),6)
        self.assertEqual([(c['id'],c['action'],c['end_sample']-c['start_sample']) for c in timeline['cues']],[('nod-em','gesture',24000)])
        self.assertGreaterEqual(timeline['cues'][0]['start_sample'],tr['fade_in_end'])
        self.assertTrue(all(f['outcome']!='pass' for f in doc['acceptance']['findings']))
        for p,h in old_hashes.items():self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),h)
        def continuous(pcm,segments):return b''.join(pcm[a*4:b*4] for a,b in [s['destination'] for s in segments])
        with wave.open(io.BytesIO((BASE/'audio/master.wav').read_bytes())) as f:old=f.readframes(f.getnframes())
        with wave.open(io.BytesIO(revision['files']['audio/master.wav'])) as f:
            self.assertEqual((f.getframerate(),f.getnchannels(),f.getsampwidth()),(48000,2,2));new=f.readframes(f.getnframes())
        self.assertEqual(continuous(old,baseline['presentation_map']['segments']),continuous(new,doc['presentation_map']['segments']))

    def test_publication_is_separate_idempotent_and_refuses_changed_existing_bytes(self):
        import tempfile
        from readiness_correction import publish_revision
        with tempfile.TemporaryDirectory(dir=ROOT/'fixtures/generation') as folder:
            output=Path(folder)/'two-scene-readiness-v2'
            first=publish_revision(ROOT,output)
            manifest=(output/'manifest.json').read_bytes()
            self.assertEqual(publish_revision(ROOT,output)['manifest_sha256'],first['manifest_sha256'])
            (output/'manifest.json').write_bytes(manifest+b' ')
            with self.assertRaises(ValueError):publish_revision(ROOT,output)
            self.assertEqual((output/'manifest.json').read_bytes(),manifest+b' ')
            (output/'manifest.json').write_bytes(manifest)
            (output/'unexpected.json').write_text('{}')
            with self.assertRaises(ValueError):publish_revision(ROOT,output)
