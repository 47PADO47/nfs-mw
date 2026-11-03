# Autosave option

The Gameplay Autosave row edits the loaded profile's native switch, not a global graphics/config preference.
The existing [save format](../formats/career-save.md), sections 2 and 6, identifies the 132-byte options block
at file offset 0x5B41 and its Autosave byte at relative offset 0x14. Preserve every other option byte.

Load the switch into the runtime autosave manager. New profiles enable it both in the file and manager.
Normal profile serialization writes the current manager value back into that byte and recomputes integrity values.
Manual saving remains available when Autosave is off; existing online, event and failure gates stay in place.

The row is disabled without a loaded profile. Changing it writes only the option into the last saved local profile,
or copies the retained imported/initial profile into the remake's own profile directory when no local file exists.
It does not checkpoint unsaved cash, progress or mod data. A failed write leaves the live option unchanged and
shows a save/load notice. Original-install save folders are never a write destination.

An explicit user choice clears a queued autosave and the error/retry flags. Turning Autosave off must not leave
an old failure prompt or pending write armed. Fresh triggers after turning it on use the existing autosave gate.
The option remains profile-specific across switching aliases and restarting. Other original options remain opaque.
