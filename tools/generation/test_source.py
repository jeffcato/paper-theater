import hashlib
import unittest
from pathlib import Path
from source import freeze_chapter

ROOT = Path(__file__).resolve().parents[2] / 'fixtures/generation/source'
SELECTORS = [
    dict(book_id='pg-55', heading='Chapter I', parent=None, body_title='The Cyclone',
         next_heading='Chapter II',
         opening='Dorothy lived in the midst of the great Kansas prairies,',
         ending='Dorothy soon closed her eyes and fell fast asleep.'),
    dict(book_id='pg-36', heading='I.', parent='BOOK ONE', next_heading='II.', body_title='THE EVE OF THE WAR.',
         opening='No one would have believed in the last years of the nineteenth century',
         ending='It seemed so safe and tranquil.'),
]

class SourceTests(unittest.TestCase):
    def test_freezes_actual_chapter_bodies_with_raw_scalar_mapping(self):
        for number, selector in zip([55,36],SELECTORS):
            raw=(ROOT/f'pg{number}-raw.txt').read_bytes()
            frozen=freeze_chapter(raw,selector)
            text=frozen['text']
            self.assertTrue(text.startswith(selector['opening']))
            self.assertTrue(text.endswith(selector['ending']))
            self.assertEqual(frozen['raw_sha256'],hashlib.sha256(raw).hexdigest())
            self.assertEqual(frozen['source_sha256'],hashlib.sha256(text.encode()).hexdigest())
            self.assertEqual(len(frozen['scalar_to_raw']),len(text))
            decoded=raw.decode()
            for char,(a,b) in zip(text,frozen['scalar_to_raw']):
                if not char.isspace(): self.assertEqual(decoded[a:b],char)
            if number==55:self.assertEqual(text,(ROOT/'pg55-expected.txt').read_text())

    def test_frozen_source_rejects_stale_hash_and_ambiguous_bodies(self):
        from source import verify_frozen
        import copy
        raw=(ROOT/'pg36-raw.txt').read_bytes()
        frozen=freeze_chapter(raw,SELECTORS[1])
        verify_frozen(raw,frozen)
        for key in ['raw_sha256','clean_sha256','source_sha256','scalar_to_raw']:
            bad=copy.deepcopy(frozen);bad[key]=[] if key=='scalar_to_raw' else '0'*64
            with self.assertRaises(ValueError):verify_frozen(raw,bad)
        body='BOOK ONE\n\nI.\n\nTHE EVE OF THE WAR.\n\n'+frozen['text']+'\n\nII.\n\nnext\n\n'
        with self.assertRaises(ValueError):freeze_chapter((body+body).encode(),SELECTORS[1])
        with self.assertRaises(ValueError):freeze_chapter(body.replace('BOOK ONE','BOOK TWO').encode(),SELECTORS[1])

    def test_formatting_markup_is_removed_without_losing_raw_scalar_provenance(self):
        raw='Chapter I\n\nTitle\n\nA _star_ burned.\n\nChapter II\n\nNext.'.encode()
        selector=dict(book_id='fixture',heading='Chapter I',body_title='Title',parent=None,next_heading='Chapter II',opening='A star burned.',ending='burned.')
        frozen=freeze_chapter(raw,selector)
        self.assertEqual(frozen['text'],'A star burned.')
        for char,(a,b) in zip(frozen['text'],frozen['scalar_to_raw']):
            if not char.isspace():self.assertEqual(raw.decode()[a:b],char)
