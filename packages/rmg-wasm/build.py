#!/usr/bin/env python3
"""Build the RMG WebAssembly module and stage the npm package."""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib

PACKAGE = Path(__file__).resolve().parent
ROOT = PACKAGE.parents[1]
sys.path.insert(0, str(ROOT))
from tools.cargo_run import build_store, owned_target


def main() -> None:
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text(encoding='utf-8'))
    version, = {entry['version'] for entry in lock['package'] if entry['name'] == 'wasm-bindgen'}
    cli = shutil.which('wasm-bindgen')
    if not cli:
        raise SystemExit(f'Install wasm-bindgen-cli {version} to build this package.')
    actual = subprocess.check_output([cli, '--version'], text=True).strip()
    if actual != f'wasm-bindgen {version}':
        raise SystemExit(f'Expected wasm-bindgen {version}; found {actual}.')

    _, namespace = build_store(ROOT)
    target = owned_target(ROOT, namespace)
    subprocess.run([
        sys.executable, '-m', 'tools.cargo_run', '--', 'build', '--locked', '-p', 'vera20k-core',
        '--bin', 'snapshot-layout', '--profile', 'fast',
    ], cwd=ROOT, check=True)
    snapshot_layout = target / 'fast' / ('snapshot-layout.exe' if os.name == 'nt' else 'snapshot-layout')
    env = {
        **os.environ,
        'VERA20K_SNAPSHOT_LAYOUT': subprocess.check_output([str(snapshot_layout)], text=True).strip(),
    }
    subprocess.run([
        sys.executable, '-m', 'tools.cargo_run', '--', 'build', '--locked', '--release',
        '-p', 'vera20k-rmg-wasm', '--target', 'wasm32-unknown-unknown', '--features', 'wasm,ares',
    ], cwd=ROOT, env=env, check=True)
    binary = target / 'wasm32-unknown-unknown' / 'release' / 'vera20k_rmg_wasm.wasm'
    if not binary.is_file():
        raise SystemExit(f'Cargo succeeded but did not produce {binary}.')

    with tempfile.TemporaryDirectory(prefix='rmg-wasm-bindgen-', dir=PACKAGE) as temporary:
        generated = Path(temporary)
        subprocess.run([cli, '--target', 'web', '--out-dir', str(generated), str(binary)], check=True)
        destination = (PACKAGE / 'dist').resolve()
        if not destination.is_relative_to(PACKAGE.resolve()):
            raise SystemExit(f'Refusing to replace output outside the package: {destination}')
        shutil.rmtree(destination, ignore_errors=True)
        (destination / 'wasm').mkdir(parents=True)
        shutil.copyfile(PACKAGE / 'index.js', destination / 'index.js')
        shutil.copyfile(PACKAGE / 'index.d.ts', destination / 'index.d.ts')
        for name in ('vera20k_rmg_wasm.js', 'vera20k_rmg_wasm.d.ts', 'vera20k_rmg_wasm_bg.wasm'):
            source = generated / name
            if not source.is_file():
                raise SystemExit(f'wasm-bindgen did not produce {source.name}.')
            shutil.copyfile(source, destination / 'wasm' / name)

    package_manifest = json.loads((PACKAGE / 'package.json').read_text(encoding='utf-8'))
    print(f"Built {package_manifest['name']} {package_manifest['version']} in {PACKAGE / 'dist'}")


if __name__ == '__main__':
    main()
