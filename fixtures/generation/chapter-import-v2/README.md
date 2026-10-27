# Synthetic complete chapter import v2

These are small synthetic source/PCM/primary/independent observations, requests, recipes and receipts. They are not actual provider responses or production evidence. Source scalar origin 700 and the three Unicode speaker turns are explicit. The chapter has 16 words, four native PCM chunks and two source facts. Gate support requires two chunks; the cross-turn fact requires three. Missing/cut/ambiguous/adverse/precision/capacity variants are generated temporarily by tests.

`producer-inputs/chapter-evidence.json` pins the complete original synthetic inputs. Its `bundle/` directory contains the streamed chunk index, bounded shared private context, unit/record files, private originals and full reconstruction report. `stage-input.json`, `stage-output.json` and `stage/` show the exact RunServices replay mapping and separately identified private candidates. No directory here is an audience export.

Reproduce and inspect offline:

```sh
PT_CHAPTER_IMPORT_EXPORT=fixtures/generation/chapter-import-v2 cargo test -p pt-production generation::acoustic_tests::chapter_import::generic_complete_chapter_replays_with_global_facts_and_zero_dispatch --offline
python3 -B -c "import sys;sys.path.insert(0,'tools/generation');from chapter_import import verify_bundle;print(verify_bundle('fixtures/generation/chapter-import-v2/producer-inputs/bundle'))"
```

The 1,200-second generic and 600-second G19 capacity WAVs are temporary test artifacts and are not committed. Exact identities and pending runtime registration are documented in `docs/implementation/generation/chapter-import-ready.json` and `chapter-import-registration.json`.
