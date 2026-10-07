"""Tests for tools/chunkdump.py using small synthetic files (no game data needed).

Run from the repo root:  python -m unittest discover tests
"""

import io
import os
import struct
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))

import chunkdump  # noqa: E402


def chunk(chunk_id, payload=b""):
    return struct.pack("<II", chunk_id, len(payload)) + payload


def jdlz(payload_stream, out_size):
    header = b"JDLZ" + bytes((0x02, 0x10, 0, 0)) + struct.pack("<II", out_size, 16 + len(payload_stream))
    return header + payload_stream


class ParseTests(unittest.TestCase):
    def test_nested_container(self):
        data = chunk(0x80134000, chunk(0x00000000) + chunk(0x00134011, b"ABCD") + chunk(0x00134012, b"12345678"))
        chunks, err = chunkdump.parse_range(data, 0, len(data))
        self.assertIsNone(err)
        self.assertEqual(len(chunks), 1)
        top = chunks[0]
        self.assertEqual(top.name, "GeometryPack")
        self.assertEqual([c.id for c in top.children], [0x00000000, 0x00134011, 0x00134012])
        self.assertEqual(top.children[1].data_offset, 8 + 8 + 8)

    def test_container_bit_with_non_chunk_payload_is_kept_as_leaf(self):
        # Claims to be a container, but the payload is 6 bytes: not a valid child chunk.
        data = chunk(0x80034100, b"\x01\x02\x03\x04\x05\x06")
        chunks, err = chunkdump.parse_range(data, 0, len(data))
        self.assertIsNone(err)
        self.assertIsNone(chunks[0].children)
        self.assertIn("did not parse", chunks[0].note)

    def test_overrun_reported(self):
        data = struct.pack("<II", 0x00134011, 100) + b"short"
        chunks, err = chunkdump.parse_range(data, 0, len(data))
        self.assertEqual(chunks, [])
        self.assertIn("overruns", err)

    def test_trailing_bytes_reported(self):
        data = chunk(0x00134011, b"ABCD") + b"\x00\x00\x00"
        chunks, err = chunkdump.parse_range(data, 0, len(data))
        self.assertEqual(len(chunks), 1)
        self.assertIn("trailing", err)


class JdlzTests(unittest.TestCase):
    def test_literals_only(self):
        # flags1 = 0x00 (8 literals), flags2 unused but still read once.
        stream = bytes((0x00, 0x00)) + b"ABCDEFGH"
        self.assertEqual(chunkdump.jdlz_decompress(jdlz(stream, 8)), b"ABCDEFGH")

    def test_short_backref_overlapping(self):
        # "AB" then flags2 bit=1 copy: b0=0x01 -> dist 2, length ((0<<4)|b1)+3 with b1=3 -> 6
        # flags1 = 0b100 (literal, literal, backref); flags2 = 0x01
        stream = bytes((0x04, 0x01)) + b"AB" + bytes((0x01, 0x03))
        self.assertEqual(chunkdump.jdlz_decompress(jdlz(stream, 8)), b"ABABABAB")

    def test_long_distance_backref(self):
        # 17 literals then a flags2 bit=0 copy: dist = ((b0&0xE0)<<3 | b1) + 17, len = (b0&0x1F)+3
        literals = bytes(range(65, 65 + 17))
        # flags1 bytes: 8 literals, 8 literals, then literal + backref (bit1)
        stream = bytes((0x00, 0x00)) + literals[:8] + bytes((0x00,)) + literals[8:16] + bytes((0x02,)) + literals[16:17]
        stream += bytes((0x00, 0x00))  # b0=0 -> len 3, dist 17
        out = chunkdump.jdlz_decompress(jdlz(stream, 20))
        self.assertEqual(out, literals + literals[:3])

    def test_bare_jdlz_blob_between_chunks(self):
        inner = chunk(0x00134011, b"ABCD")  # 12 bytes, stored as literals
        stream = bytes((0x00, 0x00)) + inner[:8] + bytes((0x00,)) + inner[8:]
        blob = jdlz(stream, len(inner))
        data = chunk(0x80134000, blob + chunk(0x00134012, b"1234"))

        chunks, err = chunkdump.parse_range(data, 0, len(data))
        self.assertIsNone(err)
        kids = chunks[0].children
        self.assertEqual([k.id for k in kids], [chunkdump.JDLZ_ID, 0x00134012])
        self.assertEqual(kids[0].blob_out, len(inner))
        self.assertIsNone(kids[0].children)

        chunks, err = chunkdump.parse_range(data, 0, len(data), inflate=True)
        self.assertIsNone(err)
        self.assertEqual([c.id for c in chunks[0].children[0].children], [0x00134011])

    def test_unwrap_raww(self):
        payload = chunk(0x00134011, b"ABCD")
        raww = b"RAWW" + bytes((0x01, 0x10, 0, 0)) + struct.pack("<II", len(payload), len(payload) + 16) + payload
        data, wrapper = chunkdump.unwrap(raww)
        self.assertEqual((data, wrapper), (payload, "RAWW"))


class CliTests(unittest.TestCase):
    def run_cli(self, data, *args):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "test.bun")
            with open(path, "wb") as f:
                f.write(data)
            out = io.StringIO()
            with redirect_stdout(out):
                status = chunkdump.main([path, *args])
            return status, out.getvalue()

    def test_tree_output_names_and_strings(self):
        data = chunk(0x80134000, chunk(0x00134011, b"\x00\x00CAR_BODY_A\x00"))
        status, out = self.run_cli(data, "-s")
        self.assertEqual(status, 0)
        self.assertIn("80134000 GeometryPack", out)
        self.assertIn('"CAR_BODY_A"', out)

    def test_summary_and_find(self):
        data = chunk(0x80134000, chunk(0x00134011, b"A") + chunk(0x00134011, b"B"))
        _, out = self.run_cli(data, "--summary")
        self.assertIn("2 distinct ids", out)
        _, out = self.run_cli(data, "--find", "0x00134011")
        self.assertIn("2 match(es)", out)
        self.assertIn("in GeometryPack", out)

    def test_limit_summarises_rest(self):
        data = b"".join(chunk(0x0003B901, b"x") for _ in range(5))
        _, out = self.run_cli(data, "-n", "2")
        self.assertIn("... 3 more: 3x 0003B901 CollisionBody", out)

    def test_vpak_is_recognised(self):
        status, out = self.run_cli(b"VPAK" + b"\x00" * 28)
        self.assertEqual(status, 0)
        self.assertIn("AttribSys VPAK", out)

    def test_extract(self):
        inner = chunk(0x00134011, b"ABCD")
        data = chunk(0x80134000, inner)
        with tempfile.TemporaryDirectory() as tmp:
            out_path = os.path.join(tmp, "x.bin")
            status, _ = self.run_cli(data, "--extract", "8", "-o", out_path)
            self.assertEqual(status, 0)
            with open(out_path, "rb") as f:
                self.assertEqual(f.read(), inner)


if __name__ == "__main__":
    unittest.main()
