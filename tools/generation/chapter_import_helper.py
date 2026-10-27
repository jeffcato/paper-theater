"""Separately registered v2 chapter-import verification; no network or provider path."""
import json
from pathlib import Path
import sys
from chapter_import import encoded, need, verify_unit, META_LIMIT


def run(envelope):
    need(set(envelope) == {'operation','root','unit'} and envelope['operation'] == 'verify_imported_chapter_unit_v2', 'helper operation')
    return verify_unit(Path(envelope['root']).resolve(strict=True), envelope['unit'])


if __name__ == '__main__':
    try:
        body=sys.stdin.buffer.read(65537);need(len(body)<=65536,'stdin limit')
        result=encoded(run(json.loads(body)));need(len(result)<=META_LIMIT,'stdout limit');sys.stdout.buffer.write(result)
    except (ValueError,KeyError,TypeError,OSError,OverflowError,ZeroDivisionError):
        print('{"error":"invalid_imported_chapter_unit_v2"}');sys.exit(1)
