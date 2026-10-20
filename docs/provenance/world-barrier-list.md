# Nearby world barrier list provenance

Implementation: `libs/blackbox-collision/src/barrier_query.rs`.
Behavior contract: [world-barrier-list](../specs/world-barrier-list.md), written
before implementation. Source is the CC0
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference at revision
`80993f35224c61d2f1b135beba76c6f8234c7940`: world manager nearby instance/barrier
list production and the barrier's authored inverse-length distance helper.
The project is already credited in NOTICE. No binary address, local path,
decompiled expression, installed asset or captured memory is in public source.

Synthetic regressions cover off-centre barriers, exact radius and asymmetric
height boundaries, authored inverse-length projection/normal scaling, transform
and surface preservation, stable distance order, group/exclusion gates, square
grid coverage/deduplication, missing packs, and supplemental ungridded packs.
These finite list-producer checks do not establish whole-step world collision,
the primitive/barrier narrow phase, or complete geometry parity.
