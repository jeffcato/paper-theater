import json
import socket
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]/'fixtures/generation/acoustic'

class AcousticTests(unittest.TestCase):
    def test_saved_replay_preserves_every_measured_point_and_group_without_network(self):
        from helper import run
        saved=json.loads((ROOT/'segmented-acoustic-report.json').read_text())
        with patch.object(socket,'socket',side_effect=AssertionError('provider dispatch')):
            result=run({'operation':'replay_acoustic','root':str(ROOT)})
        self.assertEqual(result['word_track'],saved['candidate_word_track'])
        self.assertEqual(result['highlight_groups'],saved['candidate_highlight_groups'])
        self.assertEqual((len(result['word_track']),len(result['highlight_groups'])),(176,170))
        self.assertEqual(result['provider_calls'],0)
        self.assertFalse(result['production_approved'])
        self.assertEqual(result['reveal_permissions'],[])
