# HUD reference board

Visual references for the Custom HUD ([HUD_DESIGN_SYSTEM.md](HUD_DESIGN_SYSTEM.md)). They are links, not assets: no
screenshot is packaged or redistributed with the game. Each entry says what the Custom HUD takes from it and what it
leaves.

| Reference | Take | Leave |
|---|---|---|
| [Need for Speed: Most Wanted (2005), a gameplay screenshot (MobyGames)](https://www.mobygames.com/images/shots/l/137685-need-for-speed-most-wanted-windows-screenshot-you-have-to.jpg) | A speed and gear readout that reads at a glance; warm yellow as the racing accent; standings away from the middle of the road. | The round gauge, the old icons, the bulky borders, the fixed legacy places. |
| [The MobyGames Windows gallery](https://www.mobygames.com/game/20125/need-for-speed-most-wanted/screenshots/windows/), [a pursuit shot](https://www.mobygames.com/game/20125/need-for-speed-most-wanted/screenshots/ps2/238259/) and the [WSGF widescreen notes](https://www.wsgf.org/dr/need-speed-most-wanted) | What the original shows in a pursuit, and how its HUD sits on a widescreen. | The 4:3 anchors. |
| [Diablo II: Resurrected, the automap](https://cdn.gracza.pl/galeria/galeria_duze3/bp1042529937.jpg) | A map drawn straight over the world, never a card; thin pale lines that stay readable over dark scenery. | The dungeon styling and the ornament. |
| [A minimal racing HUD](https://cdn-images-1.medium.com/v2/resize%3Afit%3A1600/1%2A2V96DCJDgaCs2IMfb0ospg.jpeg) and [Gear.Club Unlimited 2](https://cdn.cloudflare.steamstatic.com/steam/apps/1557860/ss_6e0860a6d859939b5229eab606f7bcf812c2414a.1920x1080.jpg?t=1638889704) | Small position and checkpoint marks, a compact speed and gear block, the scenery as the main picture. | Labels too small to read. |
| [A technical racing interface (Screamer)](https://nfapi.noobfeed.com/storage/gallery/55774/2-%281%29.jpg) | Crisp geometry, simple line routes, groups of information kept apart, one accent per function. | Neon, a triangle round every element, several competing accents. |

## What the board decided

- **The map is the signature.** Centred, translucent, drawn from the game's own map picture and icons
  ([specs/custom-hud/map.md](specs/custom-hud/map.md)); everything else is quiet.
- **One accent.** Signal yellow is the route, the player's action and the active choice; white is the main information,
  grey the second; green confirms; red and blue belong to the police.
- **Information has states.** Free roam is quiet, a race is precise, a pursuit is urgent; the same surfaces and lines
  serve all three, and only the emphasis changes.
