# 0015 — Menu sections: one entry model for titles, rows and actions

- **Status:** accepted (2026-11-08). Built; see the spec.
- **Milestone:** cross-cutting (the option screens of milestone 6, the mod API of milestone 12).
- **Spec:** [docs/specs/menu-sections.md](../specs/menu-sections.md) holds the model, the navigation rules, the drawing in each UI
  style and the sections of each page. This ADR records the decision only.

## Question

The Gameplay and Video pages have grown to 25 and 30 rows with no grouping, and mods add rows to the end of Gameplay. The player
cannot tell the original's rows from the rewrite's, the experimental ones, or one mod's from another's. How do the option pages get
separators without a second list implementation, and how does a mod place its rows?

## Decision

1. **One entry model in `libs/blackbox-ui` (`menu`), made of flags.** An entry is an `EntryFlags` (`highlightable`, `clickable`,
   `skip_on_scroll`, `centered`, `accent_text`) with a label and an optional action. All behaviours off is a section title; all on is
   a row or an action. A title is not a new kind of row with its own code path in the lists: it is an entry whose flags the
   navigation and the themes already read. A `Section` is a title entry plus rows, the first section has no title, and a section with
   no rows shows no title.
2. **Navigation reads the flags in one place** (`menu::nav`), so the arrows, the d-pad, the stick, the wheel, the page jumps and the
   pointer all skip what must not be selected, and no input path needs its own check.
3. **The themes draw from the flags**, not from a title type: `MenuEntry` carries `EntryFlags`; the Remastered and Custom lists centre
   an entry with `centered` and colour one with `accent_text`. The Original look draws a title with the option package's own title
   string of a free slot, with no asset and no new FNG object.
4. **The option pages decide their own sections** (`frontend/options_sections.rs`), by a row's setting, so a row is moved between
   sections in one place. Sections with the same title are one section.
5. **A mod names its section in the option declaration** (`bb.settings`, `section`), appended to the size-prefixed struct so every mod
   built before it still loads and has the default section. The default is the first, untitled section, as before. The 2012 content
   mod names its own.

## Alternatives not taken

- *A separate "separator" row type in each theme.* Three drawing paths for one concept, and the navigation would test for the type
  in every input path.
- *Naming a mod's section after the mod automatically.* It would title the section of a mod that wanted its rows in the default one,
  and the manifest name is not always the label a mod wants ("NFS Most Wanted (2012) content" against "NFS: Most Wanted (2012)").
  A mod that wants its own section says so.
- *Putting the model in `crates/nfsmw`.* It has no game names, and the Mods page, the controls and any mod screen can use it.
