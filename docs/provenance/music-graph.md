# Music graph and radio (`ea-audio::mus::graph`, `nfsmw::audio::radio`)

- **Spec:** [docs/specs/music-graph.md](../specs/music-graph.md); layout notes in
  [docs/formats/audio.md](../formats/audio.md#interactive-music-mw_musicmpf--mus).
- **Sources read for the spec (2026-10-08):**
  - The install (PC v1.3): the bytes of `SOUND/PFDATA/MW_Music.mpf` and the stream headers of `MW_Music.mus`,
    and the AttribSys classes `music` and `audiosystem` of `GLOBAL/*.bin` through `blackbox-attrib`. Only counts,
    ranges, ids and the song names are quoted in the repository; no stream data or file bytes are stored.
  - `speed.exe` (PC, unpacked): static disassembly with capstone, by a previous read-only research session, of
    the PathFinder library (node chaining, transition and router selection, event lookup, the action executor and
    its jump table) and of the EA Trax code (play lists, next-song choice). Addresses of interest were noted
    there only as labels for whoever continues; none are in the code. That session's report is not in the
    repository; its findings were re-checked on the file bytes (the layout, the 3,681 records, the 70 events, the
    song-to-chain walk and the 26 song lengths) by an independent script and by the Rust reader's tests.
  - dbalatoni13/nfsmw, <https://github.com/dbalatoni13/nfsmw> (decompiled, CC0-1.0), game side only:
    `src/Speed/Indep/Src/EAXSound/sfxctl/SFXCTL_Pathfinder5.{cpp,hpp}`, `.../CARSFX/SFXObj_Pathfinder.{cpp,hpp}`
    (the EA Trax object: `InitializeEATrax`, `GenNextMusicTrackID`, `StartLicensedMusic`, `TestToLicensed`,
    `GenMusicType`), `Frontend/Database/FEDatabase.cpp` (building the song list and the playability of each
    song), `Frontend/FEManager.cpp` (the front-end skip button), `EAXSound.cpp`, and the path library headers
    (`Libs/path/5.01.04/*.h`; the `.cpp` files there are empty stubs). Read for understanding; no code copied.
- **Implemented:** 2026-10-08, from the spec only (the decompiled sources and the disassembly were not open
  while writing the Rust). `libs/ea-audio/src/mus/graph/` (reader, events, routers, chain walker) and
  `crates/nfsmw/src/audio/radio/` (play lists, streaming player, console command).
- **Checked against the game by:**
  - Synthetic-byte unit tests for the reader, the transition and router rules, the chain walker and the play
    list rules (ordered, shuffle without replacement, refill without an immediate repeat).
  - `#[ignore]`d tests on the install (`NFSMW_GAME_DIR`): the file parses (3,681 nodes, 70 events, 123
    routers, 5 variables, node table ending at the event table), all 26 song events resolve to chains with the
    stream counts and lengths of the table below, the distinct streams of each chain form a gapless run and no stream is in two songs; and a whole-song
    decode of the chains (streamed, no NaN, no jump at a join; see the radio tests).
  - **Nothing has been listened to, and the original was not traced while it played.** Which streams the
    original really reads, in which order, and when it queues the next one are inferred.
- **Known differences from the original:**
  - Pursuit, ambience and the other event opcodes are not interpreted; only the song events are.
  - The jukebox (user-chosen songs), the profile's play state and the "EA Trax" option are not modelled; the
    defaults apply. Shuffle is a player setting.
  - No song-change chyron is drawn (the player only exposes artist and title).

## Song table

Order of the `PFMapping` array (the bit number of a song); `Defplay` is `FE` (menus), `IG` (in game) or empty
(off); "start" is the song's first node, "streams" the length of its chain, "secs" the sum of the stored
stream durations. Reproduced by an independent script and by the real-install test.

| # | Artist and title | Defplay | Start | Streams | Secs |
|---|---|---|---|---|---|
| 0 | Styles Of Beyond, Nine Thou | | 2771 | 54 | 232.8 |
| 1 | T.I. Presents The P$$C, Do Ya Thang | FE | 2625 | 83 | 251.3 |
| 2 | Rock, I Am Rock | FE | 2176 | 54 | 230.7 |
| 3 | Suni Clay, In A Hood Near You | FE | 2560 | 58 | 255.2 |
| 4 | The Perceptionists, Let's Move | FE | 1561 | 39 | 177.6 |
| 5 | Juvenile, Sets Go Up | FE | 2830 | 87 | 224.8 |
| 6 | Hush, Fired Up | FE | 1490 | 66 | 204.3 |
| 7 | DJ Spooky and Dave Lombardo, B-Side Wins Again | FE | 1605 | 106 | 278.7 |
| 8 | Celldweller feat. Styles Of Beyond, Shapeshifter | | 2713 | 51 | 205.3 |
| 9 | Lupe Fiasco, Tilted | FE | 1797 | 80 | 215.7 |
| 10 | Ils, Feed The Addiction | IG | 3101 | 69 | 245.0 |
| 11 | Celldweller, One Good Reason | IG | 1999 | 164 | 259.7 |
| 12 | Hyper, We Control | IG | 2460 | 50 | 185.0 |
| 13 | Static-X, Skinnyman | IG | 2515 | 40 | 214.8 |
| 14 | Diesel Boy + Kaos, Barrier Break | IG | 3177 | 87 | 428.8 |
| 15 | Disturbed, Decadence | IG | 2376 | 75 | 233.0 |
| 16 | The Prodigy, You'll Be Under My Wheels | IG | 1882 | 112 | 241.2 |
| 17 | The Roots and BT, Tao Of The Machine | IG | 3036 | 58 | 195.7 |
| 18 | Stratus, You Must Follow | IG | 3410 | 83 | 237.1 |
| 19 | Mastodon, Blood And Thunder | IG | 2922 | 50 | 226.2 |
| 20 | Evol Intent, Mayhem & Thinktank, Broken Sword | IG | 3273 | 130 | 370.5 |
| 21 | Bullet For My Valentine, Hand Of Blood | IG | 1716 | 76 | 202.8 |
| 22 | Paul Linford and Chris Vrenna, The Mann | IG | 3559 | 57 | 214.3 |
| 23 | Avenged Sevenfold, Blinded In Chains | IG | 2237 | 132 | 373.9 |
| 24 | Jamiroquai, Feels Just Like It Should | IG | 2977 | 50 | 205.8 |
| 25 | Paul Linford and Chris Vrenna, Most Wanted Mash Up | IG | 3498 | 56 | 223.3 |
