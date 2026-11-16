#!/usr/bin/env python3
"""Audit the real pinned export, deterministic staging and unchanged checkout."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import sunrise


def snapshot(root):
    return {str(p.relative_to(root)): (sunrise.digest(p.read_bytes()), p.stat().st_mtime_ns)
            for p in root.rglob('*') if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sunrise', type=Path, default=sunrise.ROOT.parent/'sunrise')
    args = parser.parse_args()
    checkout = args.sunrise.resolve()
    status = subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain'])
    before = {p: (checkout/p).read_bytes() for p in sunrise.COMPAT['source_sha256']}
    (sunrise.ROOT/'target').mkdir(exist_ok=True)
    with sunrise.lock(sunrise.ROOT/'target/stage.lock'):
        with tempfile.TemporaryDirectory(prefix='native-stage-test-', dir=sunrise.ROOT/'target') as scratch:
            root = Path(scratch)
            output, changed = sunrise.trait_stage(checkout, root)
            assert changed
            first = snapshot(output)
            second, changed = sunrise.trait_stage(checkout, root)
            assert second == output and not changed and first == snapshot(output)
            staged = root/'sunrise-native-mods/machine_sample'
            manifest = sunrise.tomllib.loads((staged/'sunrise-mod.toml').read_text())
            assert manifest['native']['sunrise'] == sunrise.PIN
            assert 'injection' not in manifest['native']
            assert (staged/'src/machine.rs').read_bytes() == (sunrise.ROOT/'src/machine.rs').read_bytes()
            assert (staged/'Cargo.toml').is_file()
            assert not any((staged/path).exists() for path in
                           ('native', 'rust', 'sunrise-native.json', 'horizon-mixin.toml'))
            assert not (staged/'tools').exists(), 'development helper leaked into the package'
            mismatched = root/'mismatched.lock'
            mismatched.write_text('[[package]]\nname = "libc"\nversion = "999.0.0"\n')
            try:
                sunrise.check_lock(output/'Cargo.lock', mismatched)
            except ValueError as error:
                assert 'libc 999.0.0' in str(error)
            else:
                raise AssertionError('a mismatched shared dependency was accepted')
    assert all((checkout/p).read_bytes() == content for p, content in before.items())
    assert subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain']) == status
    print(json.dumps({'status': 'ok', 'original_checkout_unchanged': True,
                      'flat_package_staged': True, 'canonical_package_writer': True,
                      'deterministic_source_bytes_and_mtimes': True}, indent=2))


if __name__ == '__main__':
    main()
