#!/usr/bin/env python3
"""Package the flat project with the pinned game's shared .hmod writer."""
import argparse
import hashlib
import json
from pathlib import Path
import sunrise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sunrise', type=Path, default=sunrise.ROOT.parent/'sunrise')
    args = parser.parse_args()
    with sunrise.lock(sunrise.ROOT/'target/stage.lock'):
        sunrise.trait_stage(args.sunrise.resolve(), sunrise.ROOT/'target/trait')
        out = sunrise.ROOT/'target/sunrise-mod'/f"{sunrise.MANIFEST['id']}.hmod"
        print(json.dumps(dict(file=str(out), size=out.stat().st_size,
                              sha256=hashlib.sha256(out.read_bytes()).hexdigest()), indent=2))


if __name__ == '__main__':
    main()
