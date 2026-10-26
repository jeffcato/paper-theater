import copy,json,unittest
from pathlib import Path

class PreflightTests(unittest.TestCase):
    def fixture(self):return json.loads((Path(__file__).resolve().parents[2]/'fixtures/generation/preflight-valid.json').read_text())
    def test_purposeful_stillness_multiple_pictures_and_reference_reuse_are_valid(self):
        from preflight import validate_plan
        result=validate_plan(self.fixture())
        self.assertEqual(result['findings'],[])
        self.assertTrue(result['preflight_ready'])
        self.assertFalse(result['production_approved'])

    def test_one_revision_packet_reports_source_visibility_ownership_action_and_review_defects(self):
        from preflight import validate_plan
        changes={
            'missing-subject':lambda p:p['beats'][0]['subjects'].append('Toto'),
            'empty-entrance':lambda p:p['pictures'][0].update(instances=[]),
            'unready-entrance':lambda p:p['pictures'][0]['ready'].update(Dorothy=1),
            'later-reference':lambda p:p['references'][0].update(permitted_source_end=21),
            'unrelated-empty-reference':lambda p:p['references'][0].update(environment_empty=True),
            'duplicate-owner':lambda p:p['pictures'][0]['instances'].append({'subject_id':'Dorothy','owner_id':'backdrop'}),
            'impossible-action':lambda p:p['actions'][0].update(interval=[900,1100]),
            'omitted-action':lambda p:p.update(actions=[]),
            'unsupported-action':lambda p:p['actions'][0].update(kind='hop'),
            'missing-opening-review':lambda p:p['review'].update(opening_picture_ids=[]),
            'missing-action-review':lambda p:p['review'].update(paired_motion_ids=[]),
            'missing-hold-review':lambda p:p['review'].update(hold_samples={}),
            'missing-image-scope':lambda p:p['review'].update(image_scopes=['style']),
            'incomplete-audio-review':lambda p:p['review'].update(audio_windows=[[0,400],[401,1000]]),
            'stale-source':lambda p:p.update(observed_source_sha256='0'*64),
            'uncertain-semantic-review':lambda p:p['review'].update(semantic_outcome='uncertain')}
        for defect,change in changes.items():
            with self.subTest(defect=defect):
                plan=self.fixture();change(plan);result=validate_plan(plan)
                self.assertFalse(result['preflight_ready'])
                self.assertIn(defect,{f['id'] for f in result['findings']})
        plan=self.fixture()
        for name,change in changes.items():
            if name!='omitted-action':change(plan)
        changes['omitted-action'](plan)
        self.assertGreater(len(validate_plan(plan)['findings']),8)
