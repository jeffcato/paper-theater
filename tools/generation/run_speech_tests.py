"""Run owned Speech behavior tests against the real compiled public library.

This excludes unrelated, concurrently edited cfg(test) modules. No production source,
manifest, lockfile, shared contract or test is replaced. Temporary harness only.
--all-generation compiles current owned generation/tests and re-exports the real
current compiled runtime API; it does not run unrelated runtime unit modules.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def run():
    arguments = sys.argv[1:]
    all_generation = '--all-generation' in arguments
    arguments = [a for a in arguments if a != '--all-generation']
    built = subprocess.run(['cargo', 'build', '-p', 'pt-production', '--lib', '--locked', '--offline', '--message-format=json'], cwd=ROOT, capture_output=True, text=True)
    if built.returncode:
        print(built.stderr, file=sys.stderr)
        for line in built.stdout.splitlines():
            try:
                message = json.loads(line)
                if message.get('reason') == 'compiler-message':
                    print(message['message'].get('rendered', ''), file=sys.stderr)
            except json.JSONDecodeError:
                pass
        return built.returncode
    libraries = {}
    for line in built.stdout.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get('reason') == 'compiler-artifact':
            for filename in row['filenames']:
                if filename.endswith(('.rlib', '.so', '.dylib')):
                    libraries[row['target']['name'].replace('-', '_')] = filename
    tests = ROOT / 'crates/pt-production/src/generation/tests.rs'
    source = tests.read_text()
    helpers = source[source.index('struct OfflineServices'):source.index('#[tokio::test]')]
    harness = '''use pt_contracts::*;
use pt_production::generation::GenerationExecutor;
use async_trait::async_trait;
use serde::Serialize;
use std::{collections::BTreeMap,sync::{Mutex,atomic::{AtomicBool,AtomicUsize,Ordering}}};
fn json<T:Serialize>(v:&T)->Result<Vec<u8>,StageError>{serde_json::to_vec_pretty(v).map_err(|e|StageError::Evidence(e.to_string()))}
fn parse<T:serde::de::DeserializeOwned>(b:&[u8])->Result<T,StageError>{serde_json::from_slice(b).map_err(|e|StageError::Evidence(e.to_string()))}
''' + helpers + '\nmod owned { use super::*; #[path=' + json.dumps(str(tests.parent/'tests/speech.rs')) + '] mod speech; }\n'
    if all_generation:
        harness = 'pub mod runtime { pub use pt_production::runtime::*; }\n#[path='+json.dumps(str(tests.parent/'mod.rs'))+'] mod generation;\n'
    with tempfile.TemporaryDirectory(prefix='pt-speech-tests-') as temp:
        path = Path(temp) / 'tests.rs'
        path.write_text(harness)
        binary = Path(temp) / 'tests'
        production_directory = Path(libraries['pt_production']).parent
        dependency_directory = production_directory if production_directory.name == 'deps' else production_directory/'deps'
        command = ['rustc', '--edition=2024', '--test', str(path), '-o', str(binary), '-L', 'dependency='+str(dependency_directory)]
        for name in ['pt_production', 'pt_contracts', 'serde', 'serde_json', 'async_trait', 'tokio'] + (['dramatic_script', 'sha2', 'tempfile'] if all_generation else []):
            command += ['--extern', name+'='+libraries[name]]
        os.environ['CARGO_MANIFEST_DIR'] = str(ROOT/'crates/pt-production')
        compiled = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        if compiled.returncode:
            print(compiled.stderr, file=sys.stderr)
            return compiled.returncode
        return subprocess.run([str(binary), *arguments], cwd=ROOT).returncode


if __name__ == '__main__':
    sys.exit(run())
