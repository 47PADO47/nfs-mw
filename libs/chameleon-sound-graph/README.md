# chameleon-sound-graph

Reader for the sound graphs of a Chameleon car bundle (`GenesysObject` resources of the sound half of the bundle):
sequences driven by a live value, the items they play (wave loops, Ginsu engine voices, mixer channel gains), the
value modulators with their curves, and the header of the `Ginsu` engine resources. Everything is read from `&Bundle`;
no file is opened. The format and what is verified: [docs/formats/chameleon-sound-graph.md](../../docs/formats/chameleon-sound-graph.md).

It also reads the sound of speed: the `SpeedSoundSet` of `SOUND/SPEEDSOUNDS.BNDL` (`SpeedSoundSet`) and the wind sequence of
`SOUND/GLOBAL.BNDL` (`SoundGraphs::wind_sequence`), see [the sound of speed](../../docs/formats/chameleon-sound-graph.md#the-sound-of-speed).
