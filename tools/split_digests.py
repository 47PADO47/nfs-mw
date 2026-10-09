"""Split the output of the native digest generator into the digest source files.

Run the generator, keep its output, then feed it to this script:

    BLACKBOX_UPDATE_DIGESTS=1 cargo test -p blackbox-render --lib native_digests \
        -- --include-ignored --nocapture > /tmp/digests.txt
    python tools/split_digests.py /tmp/digests.txt

The test prints each file between `//// FILE: <path>` and `//// END` marker lines, with the path relative to
`libs/blackbox-render/src/gpu/parity/digests/`. This script writes those files and nothing else. The test
itself never writes into the repository.
"""

import sys
from pathlib import Path

DIGEST_DIR = Path("libs/blackbox-render/src/gpu/parity/digests")
FILE_MARK = "//// FILE: "
END_MARK = "//// END"


def split(text):
    """Return {relative path: file text} for every marked block in `text`."""
    files = {}
    path = None
    lines = []
    for line in text.splitlines():
        if line.startswith(FILE_MARK):
            path = line[len(FILE_MARK):].strip()
            lines = []
            continue
        if line.startswith(END_MARK):
            if path is not None:
                files[path] = "\n".join(lines) + "\n"
            path = None
            continue
        if path is not None:
            lines.append(line)
    return files


def main(argv):
    if len(argv) != 2:
        print(__doc__)
        return 2
    files = split(Path(argv[1]).read_text())
    if not files:
        print("no digest blocks found: run the generator with --nocapture")
        return 1
    for relative, text in files.items():
        target = DIGEST_DIR / relative
        # Only ever write inside the digests directory.
        if DIGEST_DIR.resolve() not in target.resolve().parents:
            print("refusing to write outside the digest directory:", relative)
            return 1
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        print("wrote", target)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
