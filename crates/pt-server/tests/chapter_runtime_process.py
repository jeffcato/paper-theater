"""Offline R16 binary/SQLite restart check; no browser or provider transport."""
import argparse
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


def wait_for(check, seconds=45):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        result = check()
        if result:
            return result
        time.sleep(.01)
    raise AssertionError('bounded process check timed out')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--receipt', default='/tmp/pt-r16-process-receipt.json')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    recipe = '76e5cb432d7e3a1a91b1e59a6e27963e4bdf0c1e531f26d5368dda476132d598'
    request = dict(schema_version=1, book_id='pg-55', source_id='pg-55-chapter-1', mode='fixture',
                   pins=dict(recipe_sha256=recipe, profile_sha256=hashlib.sha256(b'diagnostic-profile-unavailable').hexdigest(), profile_id='chapter-diagnostic'),
                   limits=dict(money_microusd=0, requests=0, tokens=0, repairs=0, wall_time_ms=30000))
    client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    process = None
    with tempfile.TemporaryDirectory(prefix='pt-chapter-process-') as directory:
        temp = Path(directory)
        original = temp / 'original'
        shutil.copytree(root / 'fixtures/generation/chapter-import-v2/producer-inputs', original)
        data = temp / 'data'

        def start(name):
            nonlocal process
            log = temp / name
            with log.open('wb') as output:
                process = subprocess.Popen([str(root / 'target/debug/pt-server'), '--port', '0', '--data', str(data), '--chapter-inputs', str(original)], cwd=root, stdout=output, stderr=subprocess.STDOUT)

            def ready():
                assert process.poll() is None, log.read_text()
                match = re.search(r'http://127\.0\.0\.1:(\d+)', log.read_text())
                return match.group(1) if match else None
            return 'http://127.0.0.1:' + wait_for(ready)

        def http(base, path, body=None):
            headers = {'Origin': base}
            if body is not None:
                headers.update({'Content-Type': 'application/json', 'Idempotency-Key': 'chapter-process'})
            with client.open(urllib.request.Request(base + path, data=None if body is None else json.dumps(body).encode(), headers=headers), timeout=5) as response:
                return json.load(response)

        try:
            base = start('first.log')
            db = sqlite3.connect(data / 'runtime.sqlite3', timeout=5)
            # Leave the real process at the last durable checkpoint, before run finish.
            db.execute("CREATE TRIGGER interrupt_terminal BEFORE UPDATE OF state ON runs WHEN NEW.state='failed' BEGIN SELECT RAISE(ABORT,'injected terminal interruption'); END")
            db.commit()
            run = http(base, '/api/v1/runs', request)['run_id']
            count = lambda sql: db.execute(sql, (run,)).fetchone()[0]
            wait_for(lambda: count('SELECT COUNT(*) FROM chapter_units WHERE run=?') == 4)
            assert count("SELECT COUNT(*) FROM attempts WHERE run=? AND state='completed'") == 4
            assert db.execute('SELECT state FROM runs WHERE id=?', (run,)).fetchone()[0] == 'running'
            outputs_before = db.execute('SELECT unit_key,outputs FROM chapter_units WHERE run=? ORDER BY label', (run,)).fetchall()
            process.kill()
            process.wait(timeout=5)
            db.execute('DROP TRIGGER interrupt_terminal')
            db.commit()
            shutil.rmtree(original)
            base = start('restart.log')
            snapshot = wait_for(lambda: (s if (s := http(base, '/api/v1/runs/' + run))['state'] == 'failed' else None))
            assert count('SELECT COUNT(*) FROM attempts WHERE run=?') == 4
            assert db.execute('SELECT unit_key,outputs FROM chapter_units WHERE run=? ORDER BY label', (run,)).fetchall() == outputs_before
            assert count("SELECT COUNT(*) FROM events WHERE run=? AND kind='chapter_unit_reused'") == 4
            assert count("SELECT COUNT(*) FROM events WHERE run=? AND kind='chapter_unit_checkpoint'") == 4
            assert not db.execute('SELECT * FROM catalog').fetchall()
            assert not db.execute('SELECT * FROM provider_v2_attempts').fetchall()
            assert any(f['id'] == 'production-verifier-pending' for f in snapshot['findings'])
            acoustic = next(s for s in snapshot['stages'] if s['stage'] == 'acoustic')
            assert acoustic['outcome'] == 'needs_repair'
            private_id = json.loads(outputs_before[0][1])[0]['id']
            try:
                http(base, '/api/v1/artifacts/' + private_id)
                raise AssertionError('private chapter report became an HTTP artifact')
            except urllib.error.HTTPError as error:
                assert error.code == 404
            receipt = dict(schema_version=1, test='chapter_runtime_process', recipe_sha256=recipe,
                           real_server_processes=2, forced_process_terminations=1, original_inputs_removed=True,
                           helper_attempts_before_restart=4, helper_attempts_after_restart=4,
                           resumed_units=4, unit_outputs_unchanged=True, checkpoint_events=4,
                           private_artifact_http_status=404, external_provider_calls=0,
                           outcome='needs_repair', run_state=snapshot['state'], production_approved=False, publication_enabled=False)
            Path(args.receipt).write_text(json.dumps(receipt, indent=2) + '\n')
            print('R16 process restart: 4 exact units reused; 0 redispatch; private HTTP 404; NeedsRepair.')
            db.close()
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait(timeout=5)


if __name__ == '__main__':
    main()
