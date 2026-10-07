#!/usr/bin/env python3
"""Dump the bChunk tree of Need for Speed: Most Wanted (2005) data files.

Almost every MW data file (.BUN, .BIN, .LZC) is a flat sequence of chunks:

    u32 id      little-endian; bit 31 set => payload is more chunks
    u32 size    payload size in bytes (header not included)
    u8  payload[size]

Files may be wrapped in a whole-file compression header (JDLZ or RAWW),
which this tool removes before parsing. See docs/formats/bchunk.md.

Standard library only; tested with Python 3.10+.
"""

import argparse
import mmap
import os
import struct
import sys
from collections import defaultdict

from bchunk_names import chunk_name

CONTAINER_BIT = 0x80000000
HEADER = struct.Struct("<II")
PADDING_ID = 0x00000000
JDLZ_ID = 0x5A4C444A  # b"JDLZ" read as a little-endian u32


# --- compression wrappers -----------------------------------------------------

class UnsupportedCompression(Exception):
    pass


def jdlz_decompress(data):
    """Decompress a JDLZ blob (16-byte header + LZ stream).

    Header: "JDLZ", u8 0x02, u8 0x10, u16 0, u32 decompressed size,
    u32 compressed size (header included).

    The stream uses two independent flag bytes. flags1 bit 0 selects
    literal (0) or back-reference (1); for a back-reference, flags2 bit 0
    selects the encoding:
      1: length = ((b0 & 0xF0) << 4 | b1) + 3, distance = (b0 & 0x0F) + 1
      0: length = (b0 & 0x1F) + 3,            distance = ((b0 & 0xE0) << 3 | b1) + 17
    """
    if data[:4] != b"JDLZ" or data[4] != 0x02 or data[5] != 0x10:
        raise ValueError("not a JDLZ v2 blob")
    out_size, in_size = struct.unpack_from("<II", data, 8)
    end = min(in_size, len(data))
    out = bytearray(out_size)
    ip, op = 16, 0
    flags1 = flags2 = 1
    while ip < end and op < out_size:
        if flags1 == 1:
            flags1 = data[ip] | 0x100
            ip += 1
        if flags2 == 1:
            flags2 = data[ip] | 0x100
            ip += 1
        if flags1 & 1:
            b0, b1 = data[ip], data[ip + 1]
            ip += 2
            if flags2 & 1:
                length = (((b0 & 0xF0) << 4) | b1) + 3
                dist = (b0 & 0x0F) + 1
            else:
                length = (b0 & 0x1F) + 3
                dist = (((b0 & 0xE0) << 3) | b1) + 17
            length = min(length, out_size - op)
            src = op - dist
            if src < 0:
                raise ValueError(f"JDLZ back-reference before start at output 0x{op:X}")
            if dist >= length:
                out[op:op + length] = out[src:src + length]
            else:  # overlapping copy repeats the last `dist` bytes
                for i in range(length):
                    out[op + i] = out[src + i]
            op += length
            flags2 >>= 1
        else:
            out[op] = data[ip]
            op += 1
            ip += 1
        flags1 >>= 1
    if op != out_size:
        raise ValueError(f"JDLZ stream ended early: {op} of {out_size} bytes")
    return bytes(out)


def unwrap(data):
    """Return (payload, wrapper_name) with any whole-file wrapper removed."""
    magic = bytes(data[:4])
    if magic == b"JDLZ":
        return jdlz_decompress(data), "JDLZ"
    if magic == b"RAWW":
        size = struct.unpack_from("<I", data, 8)[0]
        return bytes(data[16:16 + size]), "RAWW"
    if magic in (b"HUFF", b"COMP"):
        raise UnsupportedCompression(f"{magic.decode()} compression is not implemented")
    return data, None


# --- chunk tree ---------------------------------------------------------------

class Chunk:
    __slots__ = ("offset", "id", "size", "children", "note", "blob_out", "inner")

    def __init__(self, offset, chunk_id, size):
        self.offset = offset       # offset of the 8-byte header
        self.id = chunk_id
        self.size = size           # payload size
        self.children = None       # list[Chunk] for parsed containers
        self.note = ""
        self.blob_out = None       # bare JDLZ blob: decompressed size
        self.inner = None          # bare JDLZ blob with --inflate: decompressed bytes

    @property
    def data_offset(self):
        return self.offset + 8

    @property
    def end(self):
        return self.offset + 8 + self.size

    @property
    def name(self):
        return chunk_name(self.id)


def parse_jdlz_blob(buf, pos, end, inflate):
    """A bare JDLZ blob sitting where a chunk is expected (compressed solids in
    add-on car GEOMETRY.BIN files). Its 16-byte header replaces the chunk header."""
    out_size, comp_size = struct.unpack_from("<II", buf, pos + 8)
    if comp_size < 16 or pos + comp_size > end:
        return None, f"JDLZ blob at 0x{pos:X} claims {comp_size} bytes, overruns parent end 0x{end:X}"
    blob = Chunk(pos, JDLZ_ID, comp_size - 8)
    blob.blob_out = out_size
    if inflate:
        try:
            blob.inner = jdlz_decompress(bytes(buf[pos:pos + comp_size]))
        except ValueError as e:
            blob.note = f"JDLZ blob failed to decompress ({e})"
            return blob, None
        children, err = parse_range(blob.inner, 0, len(blob.inner), inflate)
        if err:
            blob.note = f"decompressed blob did not parse as chunks ({err})"
        else:
            blob.children = children
    return blob, None


def parse_range(buf, start, end, inflate=False):
    """Parse chunks in buf[start:end]. Returns (chunks, error or None)."""
    chunks = []
    pos = start
    while pos < end:
        if end - pos < 8:
            return chunks, f"{end - pos} trailing byte(s) at 0x{pos:X}"
        chunk_id, size = HEADER.unpack_from(buf, pos)
        if chunk_id == JDLZ_ID and end - pos >= 16:
            blob, err = parse_jdlz_blob(buf, pos, end, inflate)
            if err:
                return chunks, err
            chunks.append(blob)
            pos = blob.end
            continue
        chunk = Chunk(pos, chunk_id, size)
        if chunk.end > end:
            return chunks, (f"chunk {chunk_id:08X} at 0x{pos:X} claims {size} bytes, "
                            f"overruns parent end 0x{end:X}")
        if chunk_id & CONTAINER_BIT:
            children, err = parse_range(buf, chunk.data_offset, chunk.end, inflate)
            if err:
                chunk.note = f"container did not parse as chunks ({err})"
            else:
                chunk.children = children
        chunks.append(chunk)
        pos = chunk.end
    return chunks, None


def walk(chunks, path=()):
    for c in chunks:
        yield c, path
        if c.children:
            yield from walk(c.children, path + (c,))


# --- output helpers -----------------------------------------------------------

def label(chunk_id):
    name = chunk_name(chunk_id)
    return f"{chunk_id:08X} {name}" if name else f"{chunk_id:08X} ?"


def string_hint(buf, start, end, min_len=5, max_len=48, scan=512):
    """Longest printable ASCII run near the start of the payload, for spotting names."""
    best = run = b""
    for b in bytes(buf[start:min(end, start + scan)]) + b"\0":
        if 0x20 <= b < 0x7F:
            run += bytes((b,))
        else:
            if len(run) > len(best):
                best = run
            run = b""
    return best[:max_len].decode("ascii") if len(best) >= min_len else ""


def leaf_notes(buf, chunk, show_strings, hex_bytes):
    parts = []
    head = bytes(buf[chunk.data_offset:chunk.data_offset + min(chunk.size, 64)])
    stripped = head.lstrip(b"\x11")  # MW pads aligned payloads with 0x11 bytes
    if stripped[:4] == b"JDLZ":
        parts.append("[JDLZ-compressed payload]")
    if show_strings:
        s = string_hint(buf, chunk.data_offset, chunk.end)
        if s:
            parts.append(f'"{s}"')
    if hex_bytes:
        parts.append(bytes(buf[chunk.data_offset:chunk.data_offset + min(chunk.size, hex_bytes)]).hex(" "))
    return "  ".join(parts)


def print_tree(buf, chunks, args, depth=0):
    indent = "  " * depth
    shown = [c for c in chunks if args.padding or c.id != PADDING_ID]
    limit = args.limit if args.limit > 0 else len(shown)
    for c in shown[:limit]:
        line = f"{indent}@{c.offset:08X}  {label(c.id)}  ({c.size:,} B"
        if c.blob_out is not None:
            line = f"{indent}@{c.offset:08X}  JDLZ block  ({c.size + 8:,} -> {c.blob_out:,} B"
            if c.children is not None:
                line += f", {len(c.children)} children; offsets inside are relative to the block)"
            else:
                line += ")" + ("" if args.inflate else "  [use --inflate to look inside]")
        elif c.children is not None:
            line += f", {len(c.children)} children)"
        else:
            line += ")"
            extra = leaf_notes(buf, c, args.strings, args.hex)
            if extra:
                line += "  " + extra
        if c.note:
            line += f"  !! {c.note}"
        print(line)
        if c.children and (args.depth is None or depth + 1 < args.depth):
            print_tree(c.inner if c.inner is not None else buf, c.children, args, depth + 1)
    rest = shown[limit:]
    if rest:
        groups = defaultdict(lambda: [0, 0])
        for c in rest:
            groups[c.id][0] += 1
            groups[c.id][1] += c.size
        desc = ", ".join(f"{n}x {label(i)} ({b:,} B)" for i, (n, b) in
                         sorted(groups.items(), key=lambda kv: -kv[1][0]))
        print(f"{indent}... {len(rest)} more: {desc}")


def print_summary(chunks):
    stats = defaultdict(lambda: [0, 0, set()])
    failures = 0
    for c, path in walk(chunks):
        s = stats[c.id]
        s[0] += 1
        s[1] += c.size
        s[2].add(len(path))
        failures += bool(c.note)
    print(f"{'id':>8}  {'count':>9}  {'payload bytes':>15}  depth  name")
    for chunk_id in sorted(stats):
        n, b, depths = stats[chunk_id]
        d = ",".join(str(x) for x in sorted(depths))
        print(f"{chunk_id:08X}  {n:>9,}  {b:>15,}  {d:<5}  {chunk_name(chunk_id) or '?'}")
    total = sum(s[0] for s in stats.values())
    print(f"\n{total:,} chunks, {len(stats)} distinct ids, {failures} container parse failure(s)")


def print_find(chunks, wanted):
    hits = 0
    for c, path in walk(chunks):
        if c.id == wanted:
            hits += 1
            trail = " > ".join(p.name or f"{p.id:08X}" for p in path) or "(top level)"
            print(f"@{c.offset:08X}  {c.size:>12,} B  in {trail}")
    print(f"{hits} match(es)")


# --- main ---------------------------------------------------------------------

def parse_int(text):
    return int(text, 0)


def open_buffer(path):
    size = os.path.getsize(path)
    if size == 0:
        return b"", None
    f = open(path, "rb")
    return mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ), f


def dump_file(path, args):
    print(f"== {path}")
    raw, handle = open_buffer(path)
    try:
        if len(raw) == 0:
            print("(empty file)\n")
            return 0
        try:
            buf, wrapper = unwrap(raw)
        except UnsupportedCompression as e:
            print(f"!! {e}\n")
            return 1
        if wrapper:
            print(f"   {wrapper} wrapper: {len(raw):,} -> {len(buf):,} bytes (offsets below are in the unwrapped data)")
            if args.save_unwrapped:
                with open(args.save_unwrapped, "wb") as f:
                    f.write(buf)
                print(f"   unwrapped data written to {args.save_unwrapped}")
        if bytes(buf[:4]) == b"VPAK":
            print("   AttribSys VPAK database (attributes/gameplay data), not a bChunk file.")
            print("   See docs/formats/attributes.md; edit with VltEd or Attribulator.\n")
            return 0
        chunks, err = parse_range(buf, 0, len(buf), args.inflate)
        if not chunks:
            print(f"!! not a bChunk file ({err or 'no chunks'})\n")
            return 1

        if args.extract is not None:
            # top-buffer chunks only: chunks inside inflated JDLZ blocks have block-relative offsets
            target = next((c for c, path in walk(chunks)
                           if c.offset == args.extract and not any(p.inner is not None for p in path)), None)
            if target is None:
                print(f"!! no chunk starts at 0x{args.extract:X}")
                return 1
            out_path = args.output or f"chunk_{target.offset:08X}_{target.id:08X}.bin"
            with open(out_path, "wb") as f:
                f.write(bytes(buf[target.offset:target.end]))
            print(f"wrote {label(target.id)} ({target.size + 8:,} bytes incl. header) to {out_path}")
        elif args.find is not None:
            print_find(chunks, args.find)
        elif args.summary:
            print_summary(chunks)
        else:
            print_tree(buf, chunks, args)
        if err:
            print(f"!! top level stopped early: {err}")
        print()
        return 1 if err else 0
    finally:
        if handle:
            raw.close()
            handle.close()


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0],
                                formatter_class=argparse.RawDescriptionHelpFormatter,
                                epilog="Numbers accept 0x prefixes, e.g. --find 0x80134000")
    p.add_argument("files", nargs="+", help="BUN/BIN/LZC files to read")
    p.add_argument("-d", "--depth", type=int, default=None, help="max tree depth to print (1 = top level only)")
    p.add_argument("-n", "--limit", type=int, default=40,
                   help="max children printed per container before summarising the rest (0 = no limit, default 40)")
    p.add_argument("-s", "--strings", action="store_true", help="show the first ASCII string in each leaf payload")
    p.add_argument("-x", "--hex", type=int, default=0, metavar="N", help="show the first N payload bytes of each leaf")
    p.add_argument("--padding", action="store_true", help="also print 00000000 padding chunks")
    p.add_argument("--inflate", action="store_true",
                   help="decompress bare JDLZ blocks between chunks (compressed car geometry) and show their chunks")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--summary", action="store_true", help="per-id count and size table instead of a tree")
    mode.add_argument("--find", type=parse_int, metavar="ID", help="list every chunk with this id and its parents")
    mode.add_argument("--extract", type=parse_int, metavar="OFFSET", help="write the chunk at OFFSET (header included) to a file")
    p.add_argument("-o", "--output", help="output path for --extract")
    p.add_argument("--save-unwrapped", metavar="PATH", help="write the decompressed data of a JDLZ/RAWW file")
    args = p.parse_args(argv)

    status = 0
    for path in args.files:
        status |= dump_file(path, args)
    return status


if __name__ == "__main__":
    sys.exit(main())
