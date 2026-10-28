"""R18 offline server crash/restart and private HTTP allowlist proof."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def wait_for(check):
    end = time.monotonic() + 45
    while time.monotonic() < end:
        result = check()
        if result:
            return result
        time.sleep(.01)
    raise AssertionError('bounded process check timed out')


def main():
    root = Path(__file__).resolve().parents[3]
    client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    process = None
    with tempfile.TemporaryDirectory(prefix='pt-direction-pilot-') as directory:
        temp = Path(directory)
        original = temp / 'inputs'
        registration = json.loads((root / 'fixtures/runtime/direction-pilot-v1/registration.json').read_bytes())
        for row in registration['files']:
            target = original / row['source']
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / row['source'], target)
        data = temp / 'data'

        def start(name):
            nonlocal process
            log = temp / name
            with log.open('wb') as output:
                process = subprocess.Popen([str(root / 'target/debug/pt-server'), '--port', '0', '--data', str(data), '--direction-pilot-inputs', str(original)], cwd=root, stdout=output, stderr=subprocess.STDOUT)
            def ready():
                assert process.poll() is None, log.read_text()
                match = re.search(r'http://127\.0\.0\.1:(\d+)', log.read_text())
                return match.group(1) if match else None
            return 'http://127.0.0.1:' + wait_for(ready)

        def http(base, path, body=None):
            headers = {'Origin': base}
            if body is not None:
                headers.update({'Content-Type': 'application/json', 'Idempotency-Key': 'direction-pilot-process'})
            with client.open(urllib.request.Request(base + path, data=None if body is None else json.dumps(body).encode(), headers=headers), timeout=5) as response:
                return json.load(response)

        try:
            base = start('first.log')
            db = sqlite3.connect(data / 'runtime.sqlite3', timeout=5)
            db.execute("CREATE TRIGGER interrupt_terminal BEFORE UPDATE OF state ON runs WHEN NEW.state='failed' BEGIN SELECT RAISE(ABORT,'injected terminal interruption'); END")
            db.commit()
            request = dict(schema_version=1, book_id=registration['book_id'], source_id=registration['chapter_id'], mode='fixture', pins={**registration['profile'], 'recipe_sha256': registration['recipe_sha256']}, limits=dict(money_microusd=0, requests=0, tokens=0, repairs=0, wall_time_ms=30000))
            run = http(base, '/api/v1/runs', request)['run_id']
            count = lambda sql: db.execute(sql, (run,)).fetchone()[0]
            wait_for(lambda: count('SELECT COUNT(*) FROM saved_flow_phases WHERE run=? AND result IS NOT NULL') == 6)
            assert db.execute('SELECT state FROM runs WHERE id=?', (run,)).fetchone()[0] == 'running'
            before = db.execute('SELECT phase,input,result,result_sha256 FROM saved_flow_phases WHERE run=? ORDER BY ordinal', (run,)).fetchall()
            process.kill()
            process.wait(timeout=5)
            db.execute('DROP TRIGGER interrupt_terminal')
            db.commit()
            shutil.rmtree(original)
            base = start('restart.log')
            snapshot = wait_for(lambda: (s if (s := http(base, '/api/v1/runs/' + run))['state'] == 'failed' else None))
            assert db.execute('SELECT phase,input,result,result_sha256 FROM saved_flow_phases WHERE run=? ORDER BY ordinal', (run,)).fetchall() == before
            assert count("SELECT COUNT(*) FROM events WHERE run=? AND kind='saved_flow_phase_checkpoint'") == 6
            assert not db.execute('SELECT * FROM artifacts').fetchall()
            for table in ['catalog', 'library', 'publications', 'provider_v2_attempts', 'provider_v2_queries', 'attempts']:
                assert not db.execute('SELECT * FROM ' + table).fetchall(), table
            assert any(f['id'] == 'retained-pilot-adverse' for f in snapshot['findings'])
            assert any(f['id'] == 'production-verifier-pending' for f in snapshot['findings'])
            for stage in ['direction', 'pilot']:
                assert next(s for s in snapshot['stages'] if s['stage'] == stage)['outcome'] == 'needs_repair'
            assert next(s for s in snapshot['stages'] if s['stage'] == 'speech')['outcome'] is None
            private_id = json.loads(before[-1][2])['artifacts'][0]['id']
            try:
                http(base, '/api/v1/artifacts/' + private_id)
                raise AssertionError('private Pilot artifact became public')
            except urllib.error.HTTPError as error:
                assert error.code == 404
            receipt = dict(schema_version=1, test='direction_pilot_process', real_server_processes=2, forced_process_terminations=1, original_inputs_removed=True, committed_phases_before_restart=6, committed_phases_after_restart=6, checkpoint_bytes_unchanged=True, checkpoint_events=6, external_provider_calls=0, helper_calls=0, private_artifact_http_status=404, bulk_speech_approved=False, production_approved=False, server_binary_sha256=hashlib.sha256((root/'target/debug/pt-server').read_bytes()).hexdigest())
            Path('/tmp/pt-r18-process-receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
            print(json.dumps(receipt))
        finally:
            if process is not None and process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == '__main__':
    main()
