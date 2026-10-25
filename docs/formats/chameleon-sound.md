# Chameleon sound: `.SPS` streams and the car sample resources

NFS: Most Wanted (2012) plays its audio through EA Audio Core 8.03 (strings in `NFS13.exe`), not the Black Box
sound system of the 2005 game ([audio.md](audio.md)). Reader: [`libs/ea-audio/src/sps`](../../libs/ea-audio/src/sps).
For the evidence tags see [README.md](../README.md#evidence-tags). Everything here was measured on the 2012 install
(v1.4.0.0); the layouts were read from the game's own decoder code and then checked against the files.

## `SOUND/STREAMS/*.SPS` **[verified on all 56 files]**

56 files, 108 MB. The file name is a decimal id. They hold ambience, music and speech, **not** car engines: the
ids are referenced from `HAWAII/AMBIENCES.BNDL` (the 6-channel 30 s and 60 s loops), `HAWAII/REVERBROADDATA.BNDL`, the
`TRK_UNIT*.BNDL` music units (`31189`, `31190`, `31235`) and the language bundles.

A stream is a list of big-endian chunks. The size field counts the whole chunk, header included.

| Chunk | Layout |
|---|---|
| `H` (0x48) | `H`, size (24 bits), then a bit string, most significant bit first (below) |
| `D` (0x44) | `D`, size (24 bits), samples of the chunk (u32), then frames |

The `H` bit string: version (4 bits, 1), codec (4), channels minus one (6), sample rate (18), kind (2), loop flag (1),
samples per channel (29); a set loop flag adds the loop start (32 bits); kind 2 adds 32 bits, and a loop flag with
kind 1 or 2 adds 32 bits more. A 12-byte `H` chunk is the plain form, a 20-byte one has loop data.

The codec numbers, from the game's table: 2 `P6B0` (PCM), 3 `EXm0` (EA-XMA), 4 `Xas1` (EA-XAS), 5 `EL31`
(EA Layer III v1), 6 `L32P`, 7 `L32S` (Layer III v2), 9 `Esp0` (Speex). **All 55 streams that decode are codec 5**
(one more is MPEG-2 Layer III, 16 kHz, and is not read).

### `EL31` frames **[verified: 55 streams decode with no error and to their declared length]**

A frame is one granule (576 samples per channel) of an MPEG-1 Layer III frame with the 4-byte MP3 header squeezed
into one byte and no bit reservoir. Bit by bit:

1. a flag byte: `0x00`, or `0xEE` when raw PCM follows the frame (a count, an offset and 16-bit big-endian samples
   that the game writes over the decoded samples, a splice for looping; the reader skips them);
2. a header byte: version (2 bits, 3 = MPEG-1), sample rate index (2), channel mode (2), mode extension (2);
3. the granule number (1 bit);
4. for granule 1, the four scalefactor-sharing bits of each channel;
5. per channel, the standard 59 bits of Layer III granule side information; the first 12 are `part2_3_length`;
6. the main data, `part2_3_length` bits for each channel in turn.

Then the frame ends at a byte boundary. Two granules (0, then 1) are one MPEG-1 frame, so repacking is: write the
MP3 header and side information (no reservoir, or a reservoir when a frame is bigger than 320 kbit/s allows) and the
two main data runs. `ea_audio::sps::to_mp3` does that and any MP3 decoder reads the result.

A stream of more than two channels is made of **cores**, one per stereo pair (a mono core for an odd last
channel). Inside a `D` chunk the cores' frames alternate, core 0, core 1, and so on. The first chunk of a stream has
about 47 extra samples; the decoded length is within five frames of the header's sample count on every stream.

## Car sample resources (kind `0x81`) **[verified on one car, `VEH_1085007_HI`]**

Each car bundle holds about 40 sample resources: block 0 is a 128-byte header and a whole `SPS` stream.

| Offset | Field |
|---|---|
| 0x00 | u32 `0x80` |
| 0x10 | f32 volume, 1.0 |
| 0x14 | f32 length in milliseconds |
| 0x18 | u32 size of the stream that follows the header |
| 0x24 | u32 flag, 1 |
| 0x80 | the `SPS` stream (`H` chunk first) |

They are mono, 48 kHz, codec 5, and decode with the same reader. Seven of the 40 loop (loop start 0), the rest are
one-shots. The two longest loops, 1.8 s each, have a harmonic series on a 41 Hz fundamental, which for a six-cylinder
engine with three firings per revolution is an idle of about 820 rpm. That car is the Porsche 911 Carrera S, a flat six
that idles near 800 rpm, so these are the car's own engine loops. Other loops have partials near 510, 680, 850 and
1,020 Hz (higher revs, or a whine).

**Not known yet:** which resource is which layer (idle, on or off throttle, a given rpm), the loop points and the rules
that blend them; the object that lists them in the car's gameplay data (kinds `0x14` and `0x15`) is not read.
`WAVES.BNDL` and `METALCAR.BNDL` hold the shared one-shots in the same resource layout.
