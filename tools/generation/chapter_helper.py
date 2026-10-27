"""Unregistered v1 chapter-unit verification entry point; offline files only."""
import json
from pathlib import Path
import sys
from chapter_acoustic import encoded,verify_unit,need,OUTPUT_LIMIT

def run(envelope):
    need(set(envelope)=={'operation','root','unit'} and envelope['operation']=='verify_chapter_unit','helper operation')
    return verify_unit(Path(envelope['root']).resolve(strict=True),envelope['unit'])

if __name__=='__main__':
    try:
        b=sys.stdin.buffer.read(65537);need(len(b)<=65536,'stdin limit');result=encoded(run(json.loads(b)));need(len(result)<=OUTPUT_LIMIT,'stdout limit');sys.stdout.buffer.write(result)
    except (ValueError,KeyError,TypeError,OSError,OverflowError):
        print('{"error":"invalid_chapter_unit"}');sys.exit(1)
