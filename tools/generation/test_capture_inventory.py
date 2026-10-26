import unittest
from pathlib import Path
from unittest.mock import patch
import socket

ROOT=Path(__file__).resolve().parents[2]

class CaptureInventoryTests(unittest.TestCase):
    def test_frozen_fixture_covers_openings_reveals_transitions_holds_action_and_terminal(self):
        from capture_inventory import build_inventory
        with patch.object(socket,'socket',side_effect=AssertionError('network dispatch')):
            inventory=build_inventory(ROOT)
        self.assertFalse(inventory['production_approved'])
        self.assertEqual(inventory['provider_calls'],0)
        self.assertEqual(inventory['bindings']['manifest_sha256'],'04f11a892cb775590120c371c7b8effbf6ea6460ca2d800e5f159d7ed46c1f1d')
        self.assertEqual([g['id'] for g in inventory['reveals']],['reveal-1','reveal-2'])
        self.assertEqual(inventory['transitions'][0]['swap_sample'],1464000)
        samples={f['sample'] for f in inventory['frames']}
        for boundary in [72000,96000,168000,240000,1416000,1464000,1560000,1608000,1656480,1680480,3190560]:
            self.assertTrue({boundary-1,boundary,boundary+1}<=samples,boundary)
        self.assertTrue({0,1,3262559,3262560}<=samples)
        self.assertTrue(all(type(s)==int and 0<=s<=3262560 for s in samples))
        dark=next(h for h in inventory['holds'] if h['kind']=='dark_hold')
        self.assertEqual((dark['interval'],dark['duration_samples'],dark['duration_seconds']),([1464000,1560000],96000,2))
        self.assertEqual({e['scene_id'] for e in inventory['entries']},{'scene-1','scene-2'})
        self.assertEqual(next(e for e in inventory['entries'] if e['scene_id']=='scene-2')['readiness'],'fail')
        action=inventory['actions'][0]
        self.assertEqual((action['id'],action['interval'],action['target_id']),('nod-em',[1656480,1680480],'em-2'))
        self.assertEqual(action['interior_samples'],[1662480,1668480,1674480])
        self.assertEqual(len(action['motion_pair']),2)
        self.assertEqual(inventory['terminal']['sample'],3262560)
        self.assertEqual(inventory['terminal']['expected_curtain_open_ratio'],0)
        self.assertEqual(len(inventory['requests']),len(samples)*3)
        for request in inventory['requests']:
            self.assertEqual(set(request['request']),{'manifest_digest','candidate_id','checker_sha256','player_sha256','profile_id','sample','width','height'})
            self.assertEqual(request['request']['sample'],request['sample'])

    def test_missing_samples_action_pairs_or_stale_bindings_cannot_validate(self):
        import copy
        from capture_inventory import build_inventory,validate_inventory
        good=build_inventory(ROOT)
        validate_inventory(ROOT,good)
        mutations=[
            lambda i:i['frames'].pop(0),
            lambda i:i['frames'][-1].update(sample=i['sample_count']+1),
            lambda i:i['requests'].pop(),
            lambda i:i['bindings'].update(manifest_sha256='0'*64),
            lambda i:i['bindings'].update(player_sha256='0'*64),
            lambda i:i['actions'][0].update(motion_pair=[]),
            lambda i:i['holds'][0].update(duration_samples=1),
            lambda i:i['entries'][1].update(readiness='pass'),
        ]
        for mutation in mutations:
            value=copy.deepcopy(good);mutation(value)
            with self.assertRaises(ValueError):validate_inventory(ROOT,value)

    def test_requests_are_consumable_by_actual_player_capture_api(self):
        import json,subprocess,tempfile
        from capture_inventory import build_inventory
        with tempfile.TemporaryDirectory() as folder:
            inventory=Path(folder)/'inventory.json';output=Path(folder)/'state.json'
            data=build_inventory(ROOT);inventory.write_text(json.dumps(data))
            result=subprocess.run(['node',str(ROOT/'tools/generation/probe_capture_inventory.mjs'),
                                   '--inventory',str(inventory),'--output',str(output)],
                                  cwd=ROOT,capture_output=True,text=True,timeout=30)
            self.assertEqual(result.returncode,0,result.stderr)
            report=json.loads(output.read_text())
            self.assertEqual(len(report['results']),len(data['requests']))
            self.assertEqual(report['provider_calls'],0)
            self.assertFalse(report['rendered'])
            self.assertEqual(report['action_observations'][0]['id'],'nod-em')
            end=[r for r in report['results'] if r['sample']==data['sample_count']]
            self.assertEqual(len(end),3)
            self.assertTrue(all(r['curtain_open_ratio']==0 for r in end))

    def test_separate_readiness_revision_gets_its_own_bindings_and_capture_schedule(self):
        from capture_inventory import build_inventory
        old=build_inventory(ROOT)
        new=build_inventory(ROOT,fixture_path='fixtures/generation/two-scene-readiness-v2')
        self.assertEqual(old['entries'][1]['readiness'],'fail')
        self.assertEqual(new['entries'][1]['readiness'],'pass')
        self.assertNotEqual(old['bindings']['manifest_sha256'],new['bindings']['manifest_sha256'])
        self.assertEqual(old['bindings']['fixture_sha256'],new['bindings']['parent_fixture_sha256'])
        self.assertEqual(new['transitions'][0]['fade_in_start'],1843698)
        self.assertEqual(new['actions'][0]['interval'],[1891698,1915698])
        self.assertEqual(new['terminal']['sample'],old['terminal']['sample'])
