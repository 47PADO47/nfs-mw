# Native impact integration

The integration follows [the native impact contract](../specs/native-impact-parity.md)
and the existing source-grounded [object contact](pc-object-contact.md),
[ground response](ground-contact-response.md),
[collision reactions](vehicle-collision-reactions.md) and
[police damage](cop-destruction-contacts.md) notes.

Read-only inspection of the matching original PC executable on 2026-10-20
confirmed the ground-query clamp order and separate object/ground end-frame
tasks. The original hash and response evidence are recorded in the linked
specifications. This changes integration scheduling and consumers of the existing
native solver; it introduces no replacement contact equations or mass table.

Installed-data regressions cover source construction and targeted production
contacts. They are remake regressions with original inputs, distinct from the
earlier isolated original-machine-code contact comparisons. Full matched live
original-game and remake impact/settling captures remain outstanding.
