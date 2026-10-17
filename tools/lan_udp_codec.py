#!/usr/bin/env python3
"""Codec for the UDP datagrams of Need for Speed: Most Wanted's LAN lobby.

Four datagram kinds exist. The 3-byte tag at offset 0 selects the kind (tag table
at speed.exe 0x0090D4F8):

    COb    25 bytes   identity, sent every 500 ms while the peer is in state 1
    PNG     4 bytes   keep-alive, sent every 500 ms once acknowledged
    DSC     4 bytes   disconnect
    MIC    6 + n     queued game data: n payload bytes, about 1,060 at most

COb layout (datagram offsets):

    0..2    tag b"COb"
    3       session flag (1 once the session is up)
    4..23   sender name, 20 bytes, NUL padded
    24      1 once the sender has heard from the peer before

PNG and DSC are the tag plus one more byte, written as zero. That byte is not
confirmed on the wire, so the decoders check the tag and length only.

MIC header: b"MIC" at 0..2, byte 3 zero (the NUL of the tag table entry), the
channel byte at 4, byte 5 not yet identified (written as zero, ignored when
decoding). The payload starts at offset 6 and is opaque to this codec.

Sources: docs/provenance/lan-client.md ("The 25-byte identity datagram" and the
keep-alive and data timing notes) and docs/specs/lan-server.md ("Client path").
The layout is traced from decompiled code. It is NOT yet confirmed against live
captures.

    python tools/lan_udp_codec.py --selftest

Standard library only; tested with Python 3.10+.
"""

import argparse
import sys

TAGS = (b"COb", b"DSC", b"PNG", b"MIC")
COB_LENGTH = 25
NAME_SIZE = 20
PNG_LENGTH = 4
DSC_LENGTH = 4
MIC_HEADER_LENGTH = 6
# Estimate from the peer record size (0x45c bytes, buffer at +0x30); see provenance.
MIC_MAX_PAYLOAD = 1060


class CodecError(Exception):
    pass


def identify(data):
    """Return the 3-byte tag at offset 0, or raise CodecError if it is not one of the four."""
    tag = bytes(data[:3])
    if tag not in TAGS:
        raise CodecError("unknown tag %r" % tag)
    return tag


def _check_tag(data, tag):
    if bytes(data[:3]) != tag:
        raise CodecError("tag is %r, not %r" % (bytes(data[:3]), tag))


def _check_length(data, name, length):
    if len(data) != length:
        raise CodecError("%s is %d bytes; expected %d" % (name, len(data), length))


def encode_cob(name, session, heard_before):
    """Build a 25-byte COb datagram. name is at most 20 bytes and is NUL padded."""
    if len(name) > NAME_SIZE:
        raise CodecError("name is %d bytes; the field holds %d" % (len(name), NAME_SIZE))
    if not 0 <= session <= 0xFF:
        raise CodecError("session flag out of range: %r" % session)
    buf = bytearray(COB_LENGTH)
    buf[0:3] = b"COb"
    buf[3] = session
    buf[4:4 + len(name)] = name
    buf[24] = 1 if heard_before else 0
    return bytes(buf)


def decode_cob(data):
    """Parse a 25-byte COb datagram. Returns (session, name, heard_before).

    The name has its trailing NUL padding removed.
    """
    _check_length(data, "COb", COB_LENGTH)
    _check_tag(data, b"COb")
    session = data[3]
    name = bytes(data[4:4 + NAME_SIZE]).rstrip(b"\0")
    heard_before = data[24] != 0
    return session, name, heard_before


def encode_png():
    """Build the 4-byte PNG keep-alive."""
    return b"PNG\0"


def decode_png(data):
    """Check a 4-byte PNG keep-alive. Raises CodecError if it is not one."""
    _check_length(data, "PNG", PNG_LENGTH)
    _check_tag(data, b"PNG")


def encode_dsc():
    """Build the 4-byte DSC disconnect."""
    return b"DSC\0"


def decode_dsc(data):
    """Check a 4-byte DSC disconnect. Raises CodecError if it is not one."""
    _check_length(data, "DSC", DSC_LENGTH)
    _check_tag(data, b"DSC")


def encode_mic(payload, channel=0):
    """Build a MIC datagram: the 6-byte header, then the opaque payload."""
    if len(payload) > MIC_MAX_PAYLOAD:
        raise CodecError("payload is %d bytes; the limit is %d" % (len(payload), MIC_MAX_PAYLOAD))
    if not 0 <= channel <= 0xFF:
        raise CodecError("channel out of range: %r" % channel)
    header = bytearray(MIC_HEADER_LENGTH)
    header[0:3] = b"MIC"
    header[4] = channel
    return bytes(header) + bytes(payload)


def decode_mic(data):
    """Parse a MIC datagram. Returns (channel, payload).

    Only the minimum length is checked on receive. MIC_MAX_PAYLOAD is a sender
    estimate and is not enforced here.
    """
    if len(data) < MIC_HEADER_LENGTH:
        raise CodecError("MIC is %d bytes; the header alone is %d" % (len(data), MIC_HEADER_LENGTH))
    _check_tag(data, b"MIC")
    return data[4], bytes(data[MIC_HEADER_LENGTH:])


def _raises(fn, *args):
    try:
        fn(*args)
    except CodecError:
        return True
    return False


def selftest():
    failures = 0

    def check(name, ok):
        nonlocal failures
        print("%-44s %s" % (name, "OK" if ok else "FAIL"))
        if not ok:
            failures += 1

    # COb
    name = b"Pado"
    cob = encode_cob(name, 1, True)
    check("COb is 25 bytes", len(cob) == COB_LENGTH)
    check("COb tag at offset 0", cob[0:3] == b"COb")
    check("COb session flag at offset 3", cob[3] == 1)
    check("COb name at offset 4..23, NUL padded", cob[4:24] == name + b"\0" * 16)
    check("COb heard-before at offset 24", cob[24] == 1)
    check("COb round trip", decode_cob(cob) == (1, name, True))
    check("COb heard-before clear round trips", decode_cob(encode_cob(name, 0, False)) == (0, name, False))
    full_name = b"N" * NAME_SIZE
    check("COb 20-byte name round trips", decode_cob(encode_cob(full_name, 7, True)) == (7, full_name, True))
    check("COb 21-byte name rejected", _raises(encode_cob, b"N" * 21, 1, True))
    check("COb session 256 rejected", _raises(encode_cob, name, 256, True))
    check("COb 24 bytes rejected", _raises(decode_cob, cob[:24]))
    check("COb 26 bytes rejected", _raises(decode_cob, cob + b"\0"))
    check("COb empty rejected", _raises(decode_cob, b""))
    check("COb wrong tag rejected", _raises(decode_cob, b"PNG" + cob[3:]))

    # PNG
    png = encode_png()
    check("PNG is 4 bytes", len(png) == PNG_LENGTH)
    check("PNG tag at offset 0", png[0:3] == b"PNG")
    check("PNG round trip accepted", not _raises(decode_png, png))
    check("PNG 3 bytes rejected", _raises(decode_png, png[:3]))
    check("PNG 5 bytes rejected", _raises(decode_png, png + b"\0"))
    check("PNG wrong tag rejected", _raises(decode_png, encode_dsc()))

    # DSC
    dsc = encode_dsc()
    check("DSC is 4 bytes", len(dsc) == DSC_LENGTH)
    check("DSC tag at offset 0", dsc[0:3] == b"DSC")
    check("DSC round trip", not _raises(decode_dsc, dsc))
    check("DSC 3 bytes rejected", _raises(decode_dsc, dsc[:3]))
    check("DSC 5 bytes rejected", _raises(decode_dsc, dsc + b"\0"))
    check("DSC wrong tag rejected", _raises(decode_dsc, png))

    # MIC
    payload = bytes(range(256)) * 2
    mic = encode_mic(payload, 3)
    check("MIC header is 6 bytes", len(mic) == MIC_HEADER_LENGTH + len(payload))
    check("MIC tag at offset 0", mic[0:3] == b"MIC")
    check("MIC channel at offset 4", mic[4] == 3)
    check("MIC round trip", decode_mic(mic) == (3, payload))
    check("MIC empty payload round trips", decode_mic(encode_mic(b"")) == (0, b""))
    check("MIC max payload round trips", decode_mic(encode_mic(b"\x01" * MIC_MAX_PAYLOAD))[1] == b"\x01" * MIC_MAX_PAYLOAD)
    check("MIC payload over limit rejected", _raises(encode_mic, b"\x01" * (MIC_MAX_PAYLOAD + 1)))
    check("MIC 5 bytes rejected", _raises(decode_mic, mic[:5]))
    check("MIC wrong tag rejected", _raises(decode_mic, cob))

    # Tag dispatch
    check("identify COb", identify(cob) == b"COb")
    check("identify PNG", identify(png) == b"PNG")
    check("identify DSC", identify(dsc) == b"DSC")
    check("identify MIC", identify(mic) == b"MIC")
    check("identify unknown tag rejected", _raises(identify, b"XYZ\0"))

    return 0 if failures == 0 else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--selftest", action="store_true", help="run the codec checks")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
