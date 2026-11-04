# UI options repair candidates: batch 18

This batch addresses three documented missing-option gaps. It does not close the earlier visual/input backlog.
No new playable build, commit or publication is part of this source batch.

1. **Music volumes:** separate Menu Music Volume and Game Music Volume rows. Legacy volume values continue
   to control both contexts; explicit context values retain layer priority. Context switches and live edits update
   the existing bus without restarting songs. Paused gameplay retains game gain; EA Trax previews use menu gain.
2. **Map modes:** separate Free Roam Map Mode and Race Map Mode rows. New profiles use the already documented
   Fixed/Rotating context defaults. Old explicit `minimap` values continue to control both contexts. Global HUD,
   console and orientation actions retain their common-setting meaning. Original split-screen policy is unchanged.
3. **Autosave:** add the loaded profile's native switch to Gameplay. The old loader ignored the saved option and
   the blank save disagreed with the runtime's enabled default. Import, normal serialization and new-profile defaults
   now agree. Preference-only writes update the existing native byte, preserving other options and unsaved progress.
   An explicit choice clears pending/error retry state; failed writes leave the live choice intact and show a notice.

## Validation scope

Fourteen initial regressions failed on both reference installations before implementation. Coverage includes legacy
migration, config priority, independent row values, native Autosave import/serialization and runtime music/map context.
Additional coverage exercises alias persistence, main/pause routes, failed preference writes, unchanged unsaved cash,
context switching without song restart, and the real scheduled audio system.

Focused final checks and exact source/binary/install identities are retained in the local validation record.
A source/runtime-tree or offline audio result does not establish rendered layouts, physical-controller behavior,
listening acceptance or complete original-game parity. The remaining options inventory and playtest reports stay open.
