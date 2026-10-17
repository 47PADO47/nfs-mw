# Original PC graphics detail controls

Status: static behavior research. Partial controls now follow this contract;
projection, pack exemptions and complete car usage rules remain unverified. See
[provenance](../provenance/original-graphics-detail.md) and the
[current implementation comparison](../graphics-detail.md).

## Overall Level of Detail

The main Video screen's Level of Detail is a five-step **performance preset**,
not a continuous mesh-distance multiplier. Its UI value is stored separately
from the advanced World Level of Detail and Car Geometry settings. Applying a
changed preset invokes the preset writer, then hardware capability clamps.
Opening the advanced menu retains the individual settings instead.

The following subset describes the inspected preset writer, before capability
clamps and the low-memory override. Numbers are internal option values, not
percentage labels:

| Master value | World detail | Car reflection detail | Reflection update | Road reflection | Shadow detail | Car geometry |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| 2 | 1 | 0 | 0 | 0 | 0 | 0 |
| 3 | 2 | 1 | 0 | 1 | 0 | 0 |
| 4 or 5 | 2 | 1 | 2 | 2 | 1 | 1 |

This is not the complete preset: it also changes other graphics fields. The
reflection-update capability limit is one, so its temporary preset value two
is reduced to one on a capable device. Systems with less than 400 MiB physical
memory additionally force world detail and car geometry to zero. Hardware
capability and vendor paths can restrict available settings further.

Consequently, the highest master preset does **not** imply that every advanced
slider is at its independently available maximum.

## World Level of Detail

The normal capability setup permits values zero through three. This option
combines several behaviors; it must not be implemented as only draw distance.

| Value | Texture upload mip discard | Normal-view mesh eligibility |
| --- | --- | --- |
| 0 | first two levels | coarse slot 2 |
| 1 | first level | coarse slot 2 |
| 2 | no levels | adaptive slot 0 or 2 |
| 3 | no levels | adaptive slot 0 or 2, different authored mesh filtering |

Mip discard is bounded by the available chain, reduces the allocated texture
dimensions, and has an explicit texture-category exception. This is texture
quality/residency behavior, separate from anisotropic sampling.

The ordinary player-view mesh selector first rejects projected sizes below two,
adds ten for the authored detail-bias flag, then rejects sizes below eighteen.
For a slot-zero solid with more than 39 polygons, it can choose slot two when
the camera-to-instance center distance is at least 25 and
`size / max(density, 6) < 8.7`. Otherwise world detail two/three permits slot zero;
zero/one chooses slot two. Missing selected slots remain absent.
A separate quality state above two uses a cutoff of 23 and slot two; its mapping
to ordinary user settings has not been established here.

At detail zero through two, submission suppresses instances carrying flag
`0x80000`. At detail three it instead suppresses flag `0x40000`, except when flag
`0x100` is also present. Preserve this distinction as authored filtering; the
artistic meaning of every affected object has not been classified.

Reflection and special-view flags select separate rules, including the 32-pixel
cutoff. See [scenery LOD](scenery-lod.md). The current 480-line projection scale
is still a reconstruction choice, not established by this PC option trace.

## Car Geometry

The advanced setting is a binary choice, with a capability maximum of one.
The inspected ride setup subtracts the selected value from that maximum when
initializing its minimum geometry LOD. Vehicle usage can override this range.
Shared game-code evidence separately shows projected-size selection of body
and tire LOD within the allowed range, with different thresholds for traffic.
It is therefore inaccurate to equate a renderer permanently loading LOD A with
the complete original High setting. Per-usage ranges, all relevant PC thresholds
and transition behavior require a dedicated car LOD implementation/validation.

## Reflection and shadow detail

Car Reflection Detail is an internal zero-to-three setting, distinct from the
boolean Car Reflection Update Rate. The field's historical name contains
"Enable", but the advanced detail slider writes its multi-level value.
Enabling the source cube uses 256-pixel faces; matching that resolution alone
does not reproduce its visibility, content or update rules. The inspected update
option changes scenery submission into reflection views, so it must not simply
be relabeled as a captures-per-second knob.

Road Reflection Detail permits up to three on the relevant capable PC path.
It controls a distinct projected 2D pass, not the car cube. A wetness gate queues
the road view; values above one add a vehicle pass and value three adds another
gameplay pass. The latter's complete content is not classified here. The source
target is 320 by 240 when enabled. Dry-road material arithmetic without this
capture is not an implementation of the road reflection slider.

Shadow Detail permits zero through two on the capable path. It changes shadow
production, shader technique and submission rules. It does not select
1024/2048/4096 map sizes: the inspected original producer uses a 1024 square
map. See [projected shadows](original-projected-shadows.md) for remaining caster,
vendor-depth and filter boundaries.

## Texture Filtering

The original option has three capability-clamped levels:

| Value | Minimum/magnification sampling | Mip sampling | Maximum anisotropy |
| --- | --- | --- | --- |
| 0 | linear | point | 1 |
| 1 | linear | linear | 1 |
| 2 | anisotropic where that axis is supported | linear | device maximum |

The inspected vendor exception retains linear min/mag filtering on the affected
vendor. The shader-effect parameter updater and direct D3D9 sampler setup agree
on this selection. Original anisotropy is not a hardcoded 8x constant. Explicit
8x/16x remake options are useful modern limits; exact matching depends on the
reference device's reported capability and effective sampler support.

## Validation boundary

These mappings come from static option writes, capability setup and consumers.
They do not establish live preset readback on every driver, the preferred
reference's installed option values, mod changes, or pixel equivalence.
Future controls need tests for the complete affected routes before claiming
original low/high equivalents. No game binaries or extracted assets accompany
this specification.
