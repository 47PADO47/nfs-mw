### dialogs

## The career's save notices (frontend/flow_save.rs, frontend/alias_menu.rs, frontend/name_entry.rs). $alias is the name
## of a career profile. A failure only says what failed: the reason is written to the log.

# A profile was loaded from the list.
dialogs-load-complete = Load complete.
dialogs-no-career-folder = There is no per-user folder for the careers.
dialogs-no-alias-to-save = There is no alias to save.
# The alias manager's Save, with no profile loaded.
dialogs-no-alias-to-save-first = There is no alias to save. Create or load one first.
dialogs-alias-saved = { $alias } saved.
dialogs-save-failed = The career could not be saved.
dialogs-alias-deleted = { $alias } deleted.
dialogs-delete-failed = The career could not be deleted.
dialogs-load-failed = The career { $alias } could not be loaded.
dialogs-career-not-started = The career could not be started.
dialogs-alias-not-created = The alias could not be created.
dialogs-alias-created = { $alias } created.
# The name typed for a new alias is already taken.
dialogs-alias-exists = The alias { $alias } already exists.

## A career race that was not won (frontend/post_race/retry.rs). The answers are the game's Retry and Quit.

# A race that may be given up: Retry runs it again, Quit leaves for free roam.
dialogs-retry-offer = You did not win this race. Try it again?
# A race the story goes on from: it can only be run again.
dialogs-race-lost-title = Race lost
dialogs-race-lost-required = You have to win this race to go on. It starts again.
