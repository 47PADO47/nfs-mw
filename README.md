# nfs-mw

Research notes and tools for the data formats of *Need for Speed: Most Wanted* (2005, PC).

- **Docs:** [docs/README.md](docs/README.md). Covers how models, maps, textures and animations are
  stored; each claim is tagged as verified against a real install, taken from the decomp, or
  unconfirmed.
- **Tool:** [`tools/chunkdump.py`](tools/chunkdump.py) dumps the bChunk tree of any `.BUN` / `.BIN` /
  `.LZC` file (standard-library Python 3.10+; handles JDLZ/RAWW compression).
  See [docs/tools/chunkdump.md](docs/tools/chunkdump.md).

```bash
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/CARS/BMWM3GTR/GEOMETRY.BIN" -s -d 2
python -m unittest discover tests
```

This repo contains no game files. Chunk ID names come from the CC0-licensed
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) decompilation.
