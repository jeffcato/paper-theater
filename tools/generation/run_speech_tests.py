"""Run owned Speech behavior tests against the real compiled public library.

This excludes unrelated, concurrently edited cfg(test) modules. No production source,
manifest, lockfile, shared contract or test is replaced. Temporary harness only.
"""
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import shutil

ROOT = Path(__file__).resolve().parents[2]


def run():
    arguments = sys.argv[1:]
    allow_cached = '--cached-runtime' in arguments
    arguments = [a for a in arguments if a != '--cached-runtime']
    built = subprocess.run(['cargo', 'build', '-p', 'pt-production', '--lib', '--locked', '--offline', '--message-format=json'], cwd=ROOT, capture_output=True, text=True)
    if built.returncode and not allow_cached:
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
    cached = bool(built.returncode)
    if cached:
        libraries['pt_production'] = str(ROOT/'target/debug/libpt_production.rlib')
        if not Path(libraries['pt_production']).is_file():
            print('No real compiled runtime snapshot is available.', file=sys.stderr)
            return built.returncode
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
    with tempfile.TemporaryDirectory(prefix='pt-speech-tests-') as temp:
        path = Path(temp) / 'tests.rs'
        path.write_text(harness)
        binary = Path(temp) / 'tests'
        production_directory = Path(libraries['pt_production']).parent
        dependency_directory = production_directory if production_directory.name == 'deps' else production_directory/'deps'
        os.environ['CARGO_MANIFEST_DIR'] = str(ROOT/'crates/pt-production')
        if cached:
            # Compile CURRENT owned generation source, using the real cached runtime API.
            # This is explicitly a source check; it cannot certify the unfinished native build.
            generation = Path(temp)/'generation.rs'
            snapshot = Path(temp)/'libpt_production.rlib'
            shutil.copyfile(libraries['pt_production'], snapshot)
            libraries['pt_production'] = str(snapshot)
            generation.write_text('pub mod runtime { pub use pt_production::runtime::*; }\n#[path='+json.dumps(str(tests.parent/'mod.rs'))+'] pub mod generation;\n')
            library = Path(temp)/'libpt_speech_check.rlib'
            command = ['rustc', '--edition=2024', '--crate-name', 'pt_speech_check', '--crate-type', 'rlib', str(generation), '-o', str(library), '-L', 'dependency='+temp, '-L', 'dependency='+str(dependency_directory)]
            for name in ['pt_production', 'pt_contracts', 'serde', 'serde_json', 'async_trait', 'tokio', 'dramatic_script', 'sha2']:
                command += ['--extern', name+'='+libraries[name]]
            compiled = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            if compiled.returncode:
                print(compiled.stderr, file=sys.stderr)
                return compiled.returncode
            harness = harness.replace('use pt_production::generation::GenerationExecutor;', 'use pt_speech_check::generation::GenerationExecutor;')
            path.write_text(harness)
            print('CURRENT generation source; CACHED real runtime ABI SHA256 '+hashlib.sha256(Path(libraries['pt_production']).read_bytes()).hexdigest()+'. Native build remains unverified.', flush=True)
        command = ['rustc', '--edition=2024', '--test', str(path), '-o', str(binary), '-L', 'dependency='+str(dependency_directory)]
        for name in ['pt_production', 'pt_contracts', 'serde', 'serde_json', 'async_trait', 'tokio']:
            command += ['--extern', name+'='+libraries[name]]
        if cached:
            command += ['-L', 'dependency='+temp, '--extern', 'pt_speech_check='+str(library)]
        compiled = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        if compiled.returncode:
            print(compiled.stderr, file=sys.stderr)
            return compiled.returncode
        return subprocess.run([str(binary), *arguments], cwd=ROOT).returncode


if __name__ == '__main__':
    sys.exit(run())
