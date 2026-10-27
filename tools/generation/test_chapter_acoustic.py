import json
import socket
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=ROOT/'fixtures/generation/presentation-evidence-v1/input-index.json'
AUDIO=ROOT/'fixtures/generation/acoustic/excerpt.wav'
SOURCE=ROOT/'fixtures/generation/acoustic/source.txt'

class ChapterAcousticTests(unittest.TestCase):
    def test_saved_narration_splits_without_losing_global_observations_or_cross_cut_evidence(self):
        from chapter_acoustic import produce,verify_bundle,verify_unit
        with tempfile.TemporaryDirectory() as temp,patch.object(socket,'socket',side_effect=AssertionError('provider call')):
            root=Path(temp)/'bundle'
            index=produce(AUDIO,SOURCE,EVIDENCE,root,max_frames=530000)
            self.assertEqual(len(index['chunks']),3)
            self.assertEqual(index['chunks'][0]['frames'],[0,530000])
            self.assertEqual(index['chunks'][-1]['frames'][1],1451280)
            report=verify_bundle(root)
            self.assertEqual(report['outcome'],'needs_repair')
            self.assertEqual((report['word_count'],report['highlight_group_count'],report['point_word_count']),(176,170,6))
            self.assertTrue(report['complete_pcm_coverage'])
            self.assertEqual(report['provider_calls'],0)
            unit=verify_unit(root,'units/chunk-0000.json')
            cellar=next(f for f in unit['facts'] if f['original']['id']=='cellar-instruction')
            self.assertEqual(cellar['required_chunks'],['chunk-0000','chunk-0001'])
            self.assertEqual(cellar['chapter_not_before_sample'],546720)
            self.assertEqual(cellar['outcome'],'pass')
            self.assertTrue(any(not w['complete'] for w in unit['words']))
            for name in ['strange-event','balloon-ascent','unprovided-fact']:
                self.assertIsNone(next(f for f in unit['facts'] if f['original']['id']==name)['chapter_not_before_sample'])
            self.assertEqual(verify_bundle(root),report)

    def test_invalid_context_is_rejected_before_any_evidence_read(self):
        import chapter_acoustic as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);index=json.loads(EVIDENCE.read_bytes())
            index.append({'artifact':{'file':{'path':'oversized-evidence.json','sha256':'0'*64,'bytes':c.FILE_LIMIT+1,'media_type':'application/json'}},'source':'missing.json'})
            path=root/'inputs.json';path.write_text(json.dumps(index))
            with patch.object(c,'read',wraps=c.read) as reads:
                with self.assertRaises(ValueError):c.produce(AUDIO,SOURCE,path,root/'bundle')
                self.assertEqual(reads.call_count,0)

    def test_600_second_temporary_chapter_exceeds_python_file_limit_but_each_bundle_fits(self):
        import chapter_acoustic as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);audio=root/'chapter.wav';frames=600*24000;offset=1000000
            with audio.open('wb') as f:
                f.write(c.header(24000,frames));f.truncate(44+frames*2);f.seek(44+offset*2)
                with AUDIO.open('rb') as original:original.seek(44);f.write(original.read())
            self.assertGreater(audio.stat().st_size,c.FILE_LIMIT)
            index=c.produce(audio,SOURCE,EVIDENCE,root/'bundle',evidence_offset=offset)
            self.assertEqual(index['chunks'][0]['file']['bytes'],c.FILE_LIMIT)
            self.assertEqual(index['limits']['maximum_segment_frames'],(c.FILE_LIMIT-44)//2)
            report=c.verify_bundle(root/'bundle');self.assertTrue(report['complete_pcm_coverage'])
            self.assertTrue(all(f['chapter_not_before_sample'] is None for f in report['facts']))
            self.assertEqual(sum(f['original']['outcome']=='pass' for f in report['facts']),4)
            for item in index['chunks']:
                unit=c.verify_unit(root/'bundle',f"units/{item['id']}.json")
                self.assertEqual(unit['global_frames'],item['frames'])
                if unit['words']:
                    self.assertEqual(unit['words'][0]['global_samples'][0],unit['words'][0]['original']['start_sample']+offset)
            with self.assertRaises(ValueError):c.produce(audio,SOURCE,EVIDENCE,root/'oversize',max_frames=index['limits']['maximum_segment_frames']+1,evidence_offset=offset)
            self.assertFalse((root/'oversize').exists())

    def test_missing_cross_cut_chunk_is_explicitly_unresolved_without_narrowing_global_facts(self):
        import chapter_acoustic as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'bundle';c.produce(AUDIO,SOURCE,EVIDENCE,root,max_frames=530000)
            path=root/'units/chunk-0000.json';unit=json.loads(path.read_bytes());index=json.loads((root/'chapter-index.json').read_bytes())
            missing=index['chunks'][1]['file']['path'];unit['included_chunks'].remove('chunk-0001');unit['files']=[f for f in unit['files'] if f['path']!=missing];path.write_bytes(c.encoded(unit))
            report=c.verify_unit(root,'units/chunk-0000.json');cellar=next(f for f in report['facts'] if f['original']['id']=='cellar-instruction')
            self.assertEqual(cellar['original']['outcome'],'pass');self.assertEqual(cellar['outcome'],'missing');self.assertIsNone(cellar['chapter_not_before_sample'])
            self.assertEqual(cellar['reason'],'cross_cut_evidence_missing')
            self.assertEqual(next(f for f in report['facts'] if f['original']['id']=='strange-event')['outcome'],'uncertain')

    def test_tampered_swapped_reordered_overlapping_missing_and_stale_chunks_are_rejected(self):
        import chapter_acoustic as c
        for fault in ['tamper','swap','reorder','overlap','missing','missing-file','source','receipt','recipe']:
            with self.subTest(fault=fault),tempfile.TemporaryDirectory() as temp:
                root=Path(temp)/'bundle';index=c.produce(AUDIO,SOURCE,EVIDENCE,root,max_frames=530000)
                a=root/index['chunks'][0]['file']['path'];b=root/index['chunks'][1]['file']['path']
                if fault=='tamper':
                    with a.open('r+b') as f:f.seek(100);f.write(b'\xff')
                if fault=='swap':aa=a.read_bytes();bb=b.read_bytes();a.write_bytes(bb);b.write_bytes(aa)
                if fault=='reorder':index['chunks'].reverse()
                if fault=='overlap':index['chunks'][1]['frames'][0]-=1
                if fault=='missing':index['chunks'].pop(1)
                if fault=='missing-file':b.unlink()
                if fault=='source':(root/'chapter-source.txt').write_text('changed source')
                if fault=='receipt':
                    p=root/index['receipts'][0]['path'];p.write_bytes(p.read_bytes()+b' ')
                if fault=='recipe':index['recipe_sha256']='0'*64
                (root/'chapter-index.json').write_bytes(c.encoded(index))
                with self.assertRaises((ValueError,OSError)):c.verify_bundle(root)

    def test_original_receipt_bytes_and_local_helper_replay_are_unchanged(self):
        import chapter_acoustic as c
        import subprocess
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'bundle';index=c.produce(AUDIO,SOURCE,EVIDENCE,root,max_frames=530000)
            for r in index['receipts']:
                original=ROOT/'fixtures/generation/acoustic'/r['path'];self.assertEqual((root/r['path']).read_bytes(),original.read_bytes())
            request=c.encoded({'operation':'verify_chapter_unit','root':str(root),'unit':'units/chunk-0000.json'})
            result=subprocess.run(['/usr/bin/python3','-E','-s','-S',str(ROOT/'tools/generation/chapter_helper.py')],input=request,capture_output=True,timeout=30,check=True)
            self.assertEqual(result.stdout,(root/'records/chunk-0000.json').read_bytes())
            self.assertLessEqual(len(result.stdout),c.OUTPUT_LIMIT)
            self.assertIsNone(next(r for r in index['receipts'] if r['path'].startswith('meta/'))['provider_recipe_sha256'])

    def test_source_mismatch_and_unbounded_metadata_fail_without_an_output(self):
        import chapter_acoustic as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);source=root/'changed.txt';source.write_text('unprovided source')
            with self.assertRaises(ValueError):c.produce(AUDIO,source,EVIDENCE,root/'bundle')
            self.assertFalse((root/'bundle').exists())
            index=root/'large.json'
            with index.open('wb') as f:f.truncate(c.META_LIMIT+1)
            with self.assertRaises(ValueError):c.produce(AUDIO,SOURCE,index,root/'bundle')

    def test_noninteger_sample_coordinates_are_rejected_as_invalid_metadata(self):
        import chapter_acoustic as c
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'bundle';index=c.produce(AUDIO,SOURCE,EVIDENCE,root,max_frames=530000)
            index['chunks'][0]['frames'][0]=0.0
            (root/'chapter-index.json').write_bytes(c.encoded(index))
            with self.assertRaisesRegex(ValueError,'integer'):c.verify_bundle(root)
