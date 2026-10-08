# The music graph and the radio

How the original plays a licensed song: the PathFinder graph in `MW_Music.mpf`, the events that start a
song, and the "EA Trax" play lists that choose the next one. Companion of
[audio-containers.md](audio-containers.md) (the streams) and [../formats/audio.md](../formats/audio.md#interactive-music-mw_musicmpf--mus).

- **Sources:** the bytes of `SOUND/PFDATA/MW_Music.mpf` and `.mus` of the PC v1.3 install; the unpacked
  `speed.exe` (static disassembly of the PathFinder library and the EA Trax code); the CC0 decompilation
  (`dbalatoni13/nfsmw`) for the game side only (`SFXObj_Pathfinder`, `FEDatabase`, `FEManager`, the AttribSys
  class `music`). Details: [../provenance/music-graph.md](../provenance/music-graph.md).
- **Evidence tags.** **[confirmed-by-data]**: checked on the install's files (the numbers are reproducible with
  the `#[ignore]`d tests of `ea-audio`). **[inferred]**: read in the code of the PC exe or the decomp, or
  deduced from the data, but never seen at play time. **[guess]**: a choice made to fill a gap.
- **Not available:** a play-time trace of the original (which `.mus` offsets it reads, in which order). Every
  behaviour below marked [inferred] or [guess] should be checked with one when it can be had.
- **Scope.** Licensed songs, the start-screen music and the way the player moves from one song to the next.
  The interactive pursuit music and the ambience (sections 1 to 4 and 6, section 7) use the same graph but
  react to game state; only a note on them is made in section 8. They are out of scope of this spec.

All integers are little-endian. "Node" means a record of the node table; node numbers are indexes into it.

## 1. File layout of `MW_Music.mpf` (version 5) [confirmed-by-data]

| Offset | Field |
|---|---|
| 0x0C | `u8` project index (0). The game's event ids carry `1 << (24 + index)` in their top byte |
| 0x0D / 0x0E / 0x0F / 0x10 / 0x11 | `u8` counts: tracks (1), sections (7), events (70), routers (123), variables (5) |
| 0x12 | `u16` node count (3,681) |
| 0x14 | node offset table: `u16` per node, value `* 4` is the node's byte offset |
| 0x1C | event offset table: `u16` per event, value `* 4` is the event's byte offset |
| 0x24 | variable table: 20 bytes each, a 16-byte NUL-padded name and a `u32` initial value (all 0) |
| 0x28 | router offset table: `routers + 1` `u32`, each an index in `u32` units from the start of the file |
| 0x2C to 0x38 | tracks table, tracks data, stream table, end of stream table (see `ea-audio::mus::Mpf`) |

The node table's offsets are authoritative. The nodes lie back to back from 0x1D0C to 0x178C0, the start of
the event offset table (3,681 records of `16 + 4 * transitions` bytes fill that range exactly). Variable names, in
order: `rapsheet`, `pursuitid`, `partnode`, `newnode`, `ambstate`.

## 2. Nodes [confirmed-by-data for the fields, inferred for the meaning]

A node is four `u32` and then its transitions:

```
+0  d0  bits 0-15   id: stream index + 1 for an audio node; 0 group head; 0xFFFF end; 0xFFFD fire event
        bits 16-20  controller number (0 in this file)
        bits 21-26  section number (1 to 6, see section 8)
        bits 27-31  repeat/loop field (0, or 31 on 34 end nodes; meaning unknown)
+4  d1  bits 0-11   router number (1-based, 0 none)
        bits 12-16  transition count
        bits 17-19  1 = the control value is random (never set in this file)
        bits 20-23  beats per bar (4 on audio nodes, 1 on control nodes)
        bits 24-31  bars (1)
+8  d2  bits 0-15   the group head this node belongs to (the "part")
+12 d3  fire-event nodes: the 24-bit id of the event to run; otherwise 0
+16 n * { i8 lo, i8 hi, i16 target }    target is a node number, -1 for none
```

Kinds in this file: 3,326 audio nodes (3,257 distinct stream indexes, which is every stream: 69 are used by
two nodes), 89 group heads, 89 end nodes (the node right after its head) and 177 fire-event nodes. An audio
node plays the **whole** of stream `id - 1`; its length is the stream's stored duration. Group heads, end nodes
and fire-event nodes are control nodes: they make no sound.

## 3. Choosing the next node [inferred, from the PathFinder code of the exe]

The player keeps a control value `v`, 0 to 127. When a node ends (or a branch jumps to it):

1. Scan the transitions in order; the first with `lo <= v <= hi` (signed, both inclusive) is chosen. When none
   matches, the transition whose range is nearest to `v` (the smaller of `|lo - v|` and `|hi - v|`, the first
   on a tie) is chosen. A node without transitions ends the track.
2. If the node has a router, its entries are `{ u16 key, u16 value }` (the table of section 1 gives the
   range of entries for router `r`: `[tab[r-1], tab[r])`). Each entry whose key equals the chosen target replaces
   it by the value; the last match wins. Checked on router 1: key 0x199, value 0x191.
3. A negative target ends the track.
4. Then control nodes are processed in a loop until an audio node is reached: a group head makes its group the
   current one and tells the game the node number (a progress callback); a fire-event node runs the event whose
   24-bit id it names; an end node with no transitions stops the track (the callback gets -1 and the track
   reports STOPPED).
5. An audio node starts streaming stream `id - 1`.

Songs have one transition per node covering the whole range 0 to 127, so the control value does not matter
for them [confirmed-by-data], and routers do not occur on the nodes of a song chain that was walked
(**[inferred]**: the walker applies routers anyway).

## 4. Events [confirmed-by-data for the layout, inferred for the opcodes]

An event is 20 header bytes and then its actions of 12 bytes: `u32 0, u32 0, u32 0, u32 (count << 24) | id24,
u32 0`. The game's event id is `(1 << (24 + project)) | id24`, so an id from the game data is matched on its low
24 bits; the count in the top byte of the file's copy is not part of the id. (A song's event in the attribute
data is `0x01xxxxxx`; in the file it reads `0x02xxxxxx`.)

An action is `{ u32 mask, u32 w1, u32 w2 }`. Bits 8 to 14 of `w1` are the opcode. There are 18; the one the
radio needs is **4, BRANCHTO**: jump to a node. Its `w2` holds the node number in the low 16 bits (0xFFFF is
"none"); bit 24 of `w2` is set on the "stop everything" form. The other opcodes are volume, pan and pitch
fades, waits, variables, posting and cancelling events, game callbacks and conditionals; they drive the
pursuit and ambience music and are not interpreted here.

**Every song's event has exactly two actions** [confirmed-by-data, all 26]:

```
action 0: w1 = 0x000F0400, w2 = 0x01FFFFFF    BRANCHTO none: stop what is playing
action 1: w1 = 0x00030400, w2 = 0x00FF<node>   BRANCHTO the song's first node
```

The start screen's music is event 0 (`0x02C53FC7`, the same shape).

## 5. From a song to its streams [confirmed-by-data]

`music` collection `PathEvent` (a `u32`) -> the event with the same low 24 bits -> action 1's node -> follow
the first transition of each node (through group heads and fire-event nodes) until the end node. The audio
nodes met on the way, in order, are the song's **chain**: each plays one stream, one after the other. The
stream indexes of a chain are consecutive in the `.mus` file. All 26 songs resolve; their chains hold 39 to
164 streams and add up to the lengths of the real tracks (for example 6:13.9 for Blinded In Chains, 3:53.0 for
Decadence, 3:24.3 for Fired Up). A chain is therefore the song cut into bars (about 2 to 6 s each); the
original plays them as one continuous song because the next stream is queued before the current one ends.

Fire-event nodes inside a chain were not interpreted when this was checked. The events they name (ids
0x6E7282, 0xE391AF, 0xD2E818, 0x641F27, 0x22E859) exist in the file; whether they change the flow of a song
(for example a skip of a stream) is **[inferred]** to be no: the chain lengths match the real songs.

## 6. The play lists (EA Trax) [inferred, from the decomp and the exe]

The song data are the collections of the AttribSys class `music`: `Artist`, `SongName`, `Album` (strings),
`PathEvent` (`u32`) and `Defplay` (a string). The **song list** is the `PFMapping` array of the `audiosystem`
collection that the field `LicensedMusic` of `audiosystem/0x7E4B0ED2` names: 26 `music` collections, in that
order. (The class has a 27th, `default`, a template that is not in the list.) The song's index in the list
is its bit number everywhere below. Order, titles and defaults [confirmed-by-data]: see the song table in
[../provenance/music-graph.md](../provenance/music-graph.md#song-table).

**Playability.** `Defplay` `FE` plays in the front end only, `IG` in the game only, `AL` in both; anything
else (the empty string of "Nine Thou" and "Shapeshifter") plays nowhere until the player enables it in the
jukebox. Two play lists are built from it, front end (list 0) and in game (list 1): a mask `TraxMask` with one
bit per song, the count of enabled songs, and `PlayBits`, the songs not played yet in this round (at first
equal to `TraxMask`). The jukebox screen (user choice of songs) and the profile are not in scope: the defaults
apply.

**Which list.** The front-end list is used while the game is in the front end (menus), the in-game list while
driving. Music volume 0 or the EA Trax option off means no licensed music.

**Choosing the next song** (the original runs it when a song is to start):
- If `PlayBits` is empty and songs are enabled, refill it from `TraxMask` and forget the last song.
- Ordered mode (`PBMode` 0, the default): the first song after the last one played (list order, no wrap) whose
  bit is in `PlayBits`; its bit is cleared. If none is after it, refill, forget the last song and start over.
- Shuffle mode: count the bits of `PlayBits`, pick the k-th with k random in `0..count`, clear it.
- When clearing a bit empties `PlayBits`, the mask is copied back into it with the song just played left
  cleared, so the same song never plays twice in a row at a round boundary.
- No enabled song: nothing plays licensed music (the game falls back to ambience, out of scope).

A shuffle therefore plays every enabled song once before any repeats ("shuffle without replacement").

## 7. Playing and skipping [inferred]

- A song starts by sending its `PathEvent` to the PathFinder track. The track plays the chain of section 5.
- When the track reports STOPPED (the end node was reached) the controller notices on its next update and
  starts the next song of section 6. There is no crossfade or gap handling beyond the stream-to-stream queueing.
- The player does **not** have a radio dial and no "previous song". Skipping stops the track and clears the
  events; the next update then starts the next song. The front end has a "second button" that does this
  (`Pathfinder5` with event -1), and the in-game HUD action `HUDACTION_NEXTSONG` (id 0x34) is believed to
  do the same (**[inferred]**; its handler was not found).
- The artist, title and album are shown on screen ("chyron") when the song starts, except for two start-screen
  event ids (0x01C53FC7 and 0x01C3FA91).
- After a pursuit ends, licensed music resumes after a delay of 40 s [inferred].
- Music volume 0 stops the licensed music; the front-end and in-game music volumes are separate settings.

## 8. Other sections of the graph (out of scope)

Section numbers of the head nodes [confirmed-by-data]: 1 to 4 have 11 groups each and are the four
interactive pursuit sets (the game's own table of start nodes, 0x199, 0x6A, 0x132, 0xD5 and 0, are exactly head
records 409, 106, 306, 213 and 0); 5 has 31 groups and holds the 26 songs plus the start-screen music; 6 has 14
groups and is the ambience (one per ambient zone). The pursuit music is started by event `0x026E7282` and then
steered by writing the control value 0 to 127 [inferred]; the ambience events are chains of
"if the section of the current node is k then branch to ..." conditionals. The ambient zone 13 (rap sheet) has
no event in the file [confirmed-by-data].

## 9. Decisions of the Rust implementation

- The reader is in `ea-audio::mus::graph`; the chain walker follows the first transition and applies routers
  but never runs events (section 5). A chain is a list of `(node, stream, duration)`.
- The player decodes one stream at a time, from the `.mus` file with positioned reads, and queues the next
  stream's samples before the current one is exhausted so the join is sample-exact (**[guess]**: the original
  hands streams to the hardware in the same way; the join was not heard).
- Default mode is ordered (`PlayState` 0 in the original's defaults); shuffle is a setting of the player.
- Songs start in the game when driving begins and in the front end when it exists; skip and on/off are
  console commands.

## 10. How to check it

1. With a Cheat Engine trace of the original: break on reads of `MW_Music.mus`, drive with EA Trax on, let a
   song end, then press skip. Offsets / `0x80` are stream indexes; they must follow a chain of section 5
   without gaps, the next song must be the one of section 6, and the skip must start a new chain at once.
2. By ear: play `radio` in the free camera with `radio next` several times; the joins between streams must not
   click or stutter (a stream fades to silence at both ends, so a click means a wrong join).
3. Open: the meaning of `d0` bits 27-31, `d1` bits 17-19 beyond "random", `d2` bits above 15, the u16 at 0x06,
   the 26 system properties of the conditional opcode, the two transitions whose target is 0xFFFF, and whether
   the PC build has the skip action on the HUD.
