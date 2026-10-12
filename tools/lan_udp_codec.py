#!/usr/bin/env python3
"""Codec for the 'COb' UDP datagram of Need for Speed: Most Wanted's LAN lobby.

This implements the layout that docs/specs/lan-server.md records from speed.exe's
datagram handler (0x008652D0):

    offset 0..3   header, not yet identified (written as zero)
    offset 4..6   tag b'COb' (the handler compares it at 0x0090D4F8)
    offset 8..27  five little-endian dwords, copied into the peer record at +0x10..+0x20
    offset 28     flag byte (non-zero asks the receiver to acknowledge)

The layout is inferred from the code. It is round-trip tested here but NOT verified
against live traffic. Treat it as unconfirmed until a capture matches it.

    python tools/lan_udp_codec.py --selftest

Standard library only; tested with Python 3.10+.
"""

import argparse
import struct
import sys

TAG = b"COb"
TAG_OFFSET = 4
DWORDS_OFFSET = 8
DWORD_COUNT = 5
FLAG_OFFSET = 28
LENGTH = FLAG_OFFSET + 1  # 29 bytes


class CodecError(Exception):
    pass


def encode(dwords, flag):
    """Build a COb datagram from five dwords and a flag."""
    if len(dwords) != DWORD_COUNT:
        raise CodecError("expected %d dwords, got %d" % (DWORD_COUNT, len(dwords)))
    for value in dwords:
        if not 0 <= value <= 0xFFFFFFFF:
            raise CodecError("dword out of range: %r" % value)
    buf = bytearray(LENGTH)
    buf[TAG_OFFSET:TAG_OFFSET + len(TAG)] = TAG
    struct.pack_into("<5I", buf, DWORDS_OFFSET, *dwords)
    buf[FLAG_OFFSET] = 1 if flag else 0
    return bytes(buf)


def decode(data):
    """Parse a COb datagram. Returns (dwords, flag). Raises CodecError on a bad datagram."""
    if len(data) < LENGTH:
        raise CodecError("datagram is %d bytes; expected at least %d" % (len(data), LENGTH))
    if data[TAG_OFFSET:TAG_OFFSET + len(TAG)] != TAG:
        raise CodecError("tag is %r, not %r" % (data[TAG_OFFSET:TAG_OFFSET + len(TAG)], TAG))
    dwords = struct.unpack_from("<5I", data, DWORDS_OFFSET)
    flag = data[FLAG_OFFSET] != 0
    return dwords, flag


def selftest():
    failures = 0

    def check(name, ok):
        nonlocal failures
        print("%-40s %s" % (name, "OK" if ok else "FAIL"))
        if not ok:
            failures += 1

    values = (1, 0xDEADBEEF, 0, 0x7FFFFFFF, 42)
    datagram = encode(values, True)
    check("length is 29 bytes", len(datagram) == LENGTH)
    check("tag at offset 4", datagram[TAG_OFFSET:TAG_OFFSET + 3] == TAG)
    check("round trip keeps dwords and flag", decode(datagram) == (values, True))
    check("flag clear round trips", decode(encode(values, False)) == (values, False))
    try:
        decode(b"\x00" * 4 + b"XXX" + b"\x00" * 22)
        check("wrong tag rejected", False)
    except CodecError:
        check("wrong tag rejected", True)
    try:
        decode(datagram[:10])
        check("short datagram rejected", False)
    except CodecError:
        check("short datagram rejected", True)
    try:
        encode((1, 2, 3), False)
        check("wrong dword count rejected", False)
    except CodecError:
        check("wrong dword count rejected", True)
    return 0 if failures == 0 else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[1])
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
