# Runtime model customization

The garage applies installed model parts using the existing
[car assembly rules](car-assembly.md). This is a session-local UI and assembly
feature, not a career inventory or a new source of performance values.

## Offered parts

Body kits, spoilers, hoods, roof scoops and rims use their original model slots.
The catalogue searches only the slot's installed car/shared type groups, requires
a highest-detail solid, and checks the base solid's spoiler/roof attachment marker
for aftermarket attachments. Wheels require the car's installed wheel setup.
Missing or incompatible parts are excluded; vehicles without choices retain stock.
Labels use installed language hashes when present, with body-kit numbers, carbon
suffixes and authoring names as fallbacks. Equal labels retain distinct identities.
Rims use the installed geometry's manufacturer, style number and diameter when
the parts/language database doesn't supply a useful distinct display name.
Long catalogues have a search field inside a bounded selector popup. A resizable
preview mode hides the vehicle browser so the car remains visible while selecting.

Stock means the actual installed stock slot, even when that slot's geometry is
integrated into the body. Selecting a body kit refills its dependent damage and
decal model slots, and applies the existing kit-dependent wheel offsets/widths.
Rims replace the front wheel part reused at all four corners. Brake, tire and
body physics records remain the resolved performance package's records.

## Live replacement

Changing a selector immediately submits the complete garage selection. Validate
car, parts, tuning and performance, assemble every requested model, then replace
the rig. Invalid requests leave the previous rig and applied selection intact.
Same-car replacement preserves physics, pose, velocity, damage, upgrades, tuning,
camera and consumable state; update displayed wheel mounts and tread geometry.
Position Chase at spawn even if an open garage prevents driving ticks; this only
initializes the view and does not advance the vehicle simulation.
Repainting a customized car keeps its selected models. Restoring stock parts keeps
paint, upgrades and tuning. Switching cars starts with that car's stock selections.
Paintable spoilers and roof scoops bind their installed skin placeholders to the
body colour and follow repaint requests. Carbon variants retain their authored
material/texture groups, including painted mounting pieces where present. Rim
atlases are loaded from the shared geometry's own texture pack. Regression checks
cover all installed rim diffuse atlases, exactly four replacement wheel mounts,
absence of the stock wheel solid, and release of the old wheel GPU meshes.

The aerodynamics slider uses the existing original tuning equipment rule:
body or spoiler identity must differ from the actual stock part. A factory spoiler,
hood, roof scoop, rims or paint alone does not unlock it. Retained locked tuning
values continue to apply, as specified in the tuning implementation.

## Scope and evidence

Attachment transforms, body dependencies and wheel sizing reuse the prior
assembly specification's provenance. No new original-game code was ported.
Synthetic UI and validation tests cover live selection, stock restoration and the
aerodynamics gate. Installed regressions check compatible models, attachments,
kit dependencies, four-wheel assembly and scene replacement without respawning.
Native renderer captures verify the selected parts in the world and in the UI.
Vinyls, decals, tint, part paint and career persistence are outside this model-only
change. No claim is made of full original customization-screen or skin parity.
