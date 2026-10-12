#!/usr/bin/env python3
"""Check an installed Need for Speed: Most Wanted server.dll for the LAN launcher.

Reads the game's own server.dll and server.cfg and reports:

- the three exports the launcher needs (StartServer, IsServerRunning, StopServer);
- for each byte pattern that the NFSLAN launcher patches (MIT, xan1242/NFSLAN,
  commit 1191be5), how many times it occurs and at which address;
- the timing and service keys in server.cfg.

Nothing is written. The game files are read in place and never copied into the
repo. Specs: docs/specs/lan-server.md.

Standard library only; tested with Python 3.10+.
"""

import argparse
import os
import re
import struct
import sys

# (pattern, expected address, offset from match to patch address)
# Expected addresses are the ones in NFSLAN's comments, for image base 0x10000000.
PATTERNS = [
    ("83 C4 08 50 E8 ? ? ? ? 8B 4E 7C 83 C4 08 6A 00 8B D8 68", 0x1001DC8B, 4),
    ("C7 46 14 FE FF FF FF C6 01 00", 0x10006ABE, 0),
    ("8B 4E 5C 8B 56 60 8B E8 8B 45 18 50", 0x10007294, 0),
    ("8B 82 38 0A 00 00 8D 88 70 03 00 00 51 8B 8A 6C 0D 00 00", 0x1001BEDF, 0),
    ("8B 86 D0 02 00 00 8D 8E D0 00 00 00 51 8B", 0x1000AB32, 0),
    ("8B 8E D4 02 00 00 52 8B 96 18 03 00 00", 0x1000AB03, 0),
    ("8B 86 38 0A 00 00 85 C0 BB ? ? ? ? 74 13 50 E8", 0x100099EF, 0),
    ("55 8D 4C 24 14 51 57 53 E8 ? ? ? ? 56 8D 54 24 24 68 ? ? ? ? 52 E8 ? ? ? ? 8B 44 24 50", 0x10026514, 8),
    ("C7 45 5C F4 FF 00 00 89 75 50 89 75 54 89 75 58", 0x1001363D, 7),
    ("51 89 7D 50 89 7D 54 89 7D 58 8B 4B 10", 0x10013C95, 1),
]

EXPORTS = [b"StartServer", b"IsServerRunning", b"StopServer"]


def pe_sections(data):
    """Return (image_base, [(rva, file_offset, size)]) for a PE file."""
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe + 20)[0]
    base = struct.unpack_from("<I", data, pe + 24 + 28)[0]
    sec_off = pe + 24 + opt_size
    secs = []
    for i in range(nsec):
        vsize, rva, raw_size, raw_ptr = struct.unpack_from("<IIII", data, sec_off + i * 40 + 8)
        secs.append((rva, raw_ptr, raw_size))
    return base, secs


def file_to_va(base, secs, off):
    for rva, raw_ptr, raw_size in secs:
        if raw_ptr <= off < raw_ptr + raw_size:
            return base + rva + (off - raw_ptr)
    return None


def compile_pattern(text):
    """Turn 'AA ? BB' into (regex bytes) that matches exactly those bytes."""
    parts = []
    for tok in text.split():
        if tok == "?":
            parts.append(b".")
        else:
            parts.append(re.escape(bytes([int(tok, 16)])))
    return re.compile(b"".join(parts), re.DOTALL)


def read_cfg(path):
    keys = {}
    with open(path, "r", encoding="latin-1") as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, _, value = line.partition("=")
            keys[key.strip()] = value.strip()
    return keys


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[1])
    parser.add_argument("install", help="game folder containing server.dll and server.cfg")
    args = parser.parse_args(argv)

    dll_path = os.path.join(args.install, "server.dll")
    cfg_path = os.path.join(args.install, "server.cfg")
    if not os.path.isfile(dll_path):
        print("ERROR: server.dll not found in %s" % args.install, file=sys.stderr)
        return 1

    with open(dll_path, "rb") as fh:
        data = fh.read()
    base, secs = pe_sections(data)
    print("server.dll: %d bytes, image base %08X" % (len(data), base))

    missing = [e.decode() for e in EXPORTS if e not in data]
    print("exports: %s" % ("all present" if not missing else "MISSING " + ", ".join(missing)))

    all_ok = not missing
    for text, expected, offset in PATTERNS:
        rx = compile_pattern(text)
        hits = []
        for m in rx.finditer(data):
            va = file_to_va(base, secs, m.start())
            if va is not None:
                hits.append(va)
        found = [h + offset for h in hits]
        ok = len(hits) == 1 and found[0] == expected
        all_ok = all_ok and ok
        status = "OK" if ok else "MISMATCH"
        print("  %08X  %-8s hits=%d  %s" % (expected, status, len(hits), ", ".join("%08X" % f for f in found)))

    if os.path.isfile(cfg_path):
        cfg = read_cfg(cfg_path)
        print("server.cfg:")
        for key in ["PINGTIME", "PINGWAIT", "TIMEIDLE", "TIMEMAXM", "LAN_THROTTLE", "TRUST", "ADDR", "ACCOUNT", "MASTER", "SLAVE", "REDIR"]:
            if key in cfg:
                print("  %-13s %s" % (key, cfg[key]))
    else:
        print("server.cfg: not found")

    print("patch set matches NFSLAN" if all_ok else "patch set does NOT match NFSLAN for this build")
    return 0 if all_ok else 2


if __name__ == "__main__":
    sys.exit(main())
