# The interactive music: pursuit sets and the state that drives them

How the Rust game decides what music plays besides the licensed songs, and what it knows about the way the
original steers the pursuit music. Companion of [music-graph.md](music-graph.md) (the graph and the songs).

- **Sources:** the same as [music-graph.md](music-graph.md) and [../provenance/music-graph.md](../provenance/music-graph.md);
  nothing new was read for this file. The behaviour of the original below is what that spec records; the rest is
  a design of this project and says so.
- **Evidence tags:** **[confirmed-by-data]** checked on the install's files, **[inferred]** read in the code of
  the exe or the decomp, **[guess]** a choice made to fill a gap, **[design]** how this project is built.
- **Not available:** a play-time trace of the original, and any listening of the result. Nothing in the
  pursuit music has been heard.

## 1. What the original does [inferred]

- The pursuit music is the same PathFinder graph as the songs, sections 1 to 4: four **sets**, each of 11
  groups of bars. The game starts it by sending the event `0x026E7282` with a property (1 to 4) that names the
  set; the event branches to the head of that set. The game's table of start nodes is `0x199, 0x6A, 0x132,
  0xD5` and a fifth entry, `0`; the four heads are in sections 1 to 4 [confirmed-by-data].
- A song's last bars contain the same event, so a pursuit can take over from a song at an exit point
  (music-graph.md section 5). The Rust game does not do this: it stops the song and starts the pursuit set.
- The game steers the music by writing a **control value**, 0 to 127. Each time a bar ends, the node's
  transition for the current value picks the next bar (music-graph.md section 3); a set's bars are therefore the
  "intensity layers": calmer and tenser bars are reached by a lower or higher value.
- Five map variables exist (`rapsheet`, `pursuitid`, `partnode`, `newnode`, `ambstate`), and the event actions
  fade, wait, set variables and test 26 system properties. **None of that is interpreted here**: what each
  variable is for, and what the fades and waits do, is unknown.
- After a pursuit the licensed songs resume after 40 s (music-graph.md section 7).

## 2. What the game tells the music [design]

`MusicInput` (a Bevy resource) holds a `MusicState`: an optional **pursuit** (a set, 1 to 4, and an intensity,
0 to 1) and a **racing** flag. The code of milestone 7 calls its hooks: `start_pursuit(set, intensity)`,
`set_pursuit_intensity`, `set_pursuit_set`, `end_pursuit`, `start_race`, `end_race`; `pursuit_set_for_heat` is
a helper that spreads the four sets over heat levels 1 to 5 **[guess]**. Until milestone 7 nothing calls them. The
`music` console command forces a state for testing (`music pursuit 3 80`, `music intensity 20`, `music clear`,
`music` for the status).

Races play the licensed songs, as free roam does; `racing` is kept for a race-specific behaviour and changes
nothing yet. The data the hooks need (heat, cops chasing, escape progress) does not exist before milestone 7.

## 3. The director [design, numbers are guesses]

Each frame, with `driving` true while a game is on (driving or paused):

1. No game: the pursuit music is dropped and the songs are free (the radio decides for itself).
2. A pursuit starts: its set becomes active and the control value is the intensity (0 to 1 scaled to 0 to 127)
   at once; the songs are taken off.
3. The control value follows the intensity at 40 units/s upwards and 8 units/s downwards, so a lull does not
   drop the music and a spike raises it within about three seconds.
4. A different set must be asked for for 2 s before the music switches, so a flickering heat level does not
   flip it.
5. The pursuit ends: the set is released and the songs stay off for **40 s** [inferred], then the radio starts a
   new song (the in-game list).

## 4. Playing and cross-fading [design]

- A pursuit set plays on its own child track of the music group, so `music_volume` and `master_volume` apply and
  a fade is a volume tween of that track.
- A decoder thread follows the graph with `ea-audio::mus::graph::Cursor`: it enters the set at its head, decodes
  one bar's stream from `MW_Music.mus`, then reads the control value and takes the next transition. It runs
  about five blocks (a second) ahead, so a change of the value is heard a second or two later.
- Songs to pursuit: the song stops with the radio's short fade-out and the set fades in over 1 s. Set to set:
  both play, the old fades out and the new fades in over 2.5 s. Pursuit to songs: the set fades out over 3 s;
  the songs return 40 s after the end of the pursuit.
- If a set cannot play (no sound device, no music file, a decoding error, a track that reaches its end) the songs
  are left on air and the set is not retried until the pursuit is over.

## 5. Differences from the original

- Events are not run: no fades, waits, conditionals or variables; fire-event nodes passed on the way are
  logged at debug level. The set is entered at its head and steered by transitions alone.
- A pursuit does not wait for an exit point of the song; the song is cut.
- The ambience sections (section 6, one group per ambient zone, started by conditionals on `ambstate`) are not
  played: no ambient zone data exists in the game yet.
- Which pursuit set follows from which heat level, how the intensity is derived from the chase, the fade times,
  the slew rates and the 2 s hold are this project's choices.

## 6. How to check it

1. By ear: `music pursuit 1 20` in a drive, then `music intensity 100`, `music pursuit 4`, `music clear`. The
   set must come in without a click, the bars must join without gaps, a higher intensity must bring tenser bars
   within a few bars, and the songs must return 40 s after `music clear`.
2. `NFSMW_GAME_DIR=... cargo test --release -p nfsmw interactive::tests::real -- --ignored --nocapture` walks
   each set at control 0, 64 and 127 and prints the bars met and how the walk ends.
3. Open: what the ends of a set mean (does a set ever stop by itself?), the property that picks the set, the
   effect of the five variables, and a trace of the original for comparison.
