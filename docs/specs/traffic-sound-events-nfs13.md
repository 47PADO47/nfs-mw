# NFS13.exe car-sound pass: the step log

Companion to [traffic-sound-events-data.md](traffic-sound-events-data.md). Each step records one reading of `NFS13.exe`.

**`NFS13.exe` car-sound pass, step 1 (read).** The exact `Genesys.Gen.VehicleSound` string is absent from `NFS13.exe`, but the
shorter `VehicleSound` string (VA `0xF9C2A8`) has one code reference, at `0x42B860`. That site is a debug assertion (it pushes the
string and a line number `0x207` and calls the logger `0xACFF30`), not a loader. The routine at `0x42B870` is a generic keyed-table
lookup: it scans a `u32` key array at `[ecx+4]` (count at `[ecx]`) and returns the matching 12-byte entry at `base + index*12`.
Next: find the callers of `0x42B870` to see which table holds the car-sound types.

**`NFS13.exe` car-sound pass, step 2 (read).** The keyed-table lookup `0x42B870` has 11 callers (`0x4A5FFB`, `0x539D16`, `0x539D36`,
`0x57C4D4`, `0x57FEA0`, `0x581BF7`, `0x583B15`, `0x584FA4`, `0x5881E4`, `0x5882D4`, `0x5883C4`). The first, at `0x4A5FFB`, passes a key read
from the object's `+0x30` field and searches the table at `[edi+0x44]`. It then scales float parameters (`+0x5C`..`+0x68`) by the
per-entry value. So this is the audio-parameter lookup, keyed by the object's type key, and the car-type table is one of the 11
callers' tables. Next: identify which caller searches the car-type (sound-owner) table.

**`NFS13.exe` car-sound pass, step 3 (read).** The other ten callers of `0x42B870` build their table the same way: the table
header is `[object + [object+0x188] + 0x1C]`, a per-object indirection at `+0x188`, and the key is pushed by the caller. Callers
`0x57FEA0`, `0x581BF7`, `0x583B15`, `0x584FA4`, `0x5881E4`, `0x5882D4` and `0x5883C4` read the owner through `[+0x184]`/`[+0x28]`,
so these are nested sound objects. The car-type table is one of these, but which one is not yet identified from the call sites
alone.

**`NFS13.exe` car-sound pass, step 4 (read).** The nested-sound initializer at `0x57C120` takes a pair from its argument and
stores the first value at `+0x184` (the owner object) and the second at `+0x188` (the table), and it sets the flag at `+0x18`.
Its callers are `0x57E87B` and `0x5EFC4E`. Next: read those two callers to see which owner and table they pass, which identifies
the car-type table.

**`NFS13.exe` car-sound pass, step 5 (read).** Two callers of the nested-sound initializer `0x57C120`:
- `0x57E87B`: copies a list of 16-bit values from the caller's argument into the object at `+0x1870`, then passes the pair
  pointer from its own argument. The `u16` list is a per-object index table, not yet identified.
- `0x5EFC4E`: works on an array of 0x200-byte slots starting at `+0x69D30`, indexed by an `eax` value. It builds the pair with
  `0x57CA50` at `esp+0x24`, then calls the initializer. So the nested sound object is one per slot in this array.
Next: read `0x57CA50` to see how the pair (owner and table) is built, which identifies the car-type table.

**`NFS13.exe` car-sound pass, step 6 (read).** The pair builder `0x57CA50` loops over rows of 0x90 bytes (four rows, up to `0x240`),
and for each of four literal tags compares 2-byte chunks of the row's name at `[edi+0x82]`. The four tags, read as dwords at
`0xFAD4F8`, `0xFAD504`, `0xFAD510` and `0xFAD51C`, are `Fron`, `Fron`, `Rear`, `Rear`: front and rear position tags. The routine
writes the matched index into a 4-by-4 grid. So this is a front and rear wheel or axle position matcher, not the car-type table.
The car-type table is therefore not in the slot-pair path, and the nested objects under `0x57C120` hold part and wheel data.

**`NFS13.exe` car-sound pass, step 7 (read): the car-type table is `VehicleListEntry`.** The sound and vehicle strings of
`NFS13.exe` include the record type `VehicleListEntry` (VA `0xF9B0AC`, repeated per field), with the fields `Vehicle ID`,
`Livery Vehicle ID`, `Vehicle Name`, `Vehicle Description`, `Vehicle Name Image`, `Vehicle Short Name`, `Vehicle Image`,
`Vehicle Power`, `Vehicle Power RPM`, `Vehicle Year`, `Vehicle Tier`, and `Car Select Sound` (VA `0xF9B3F0`, with `CarSelectSound`
at `0xF9B404`). The list type is `VehicleListResourceType` (VA `0xF9B5E0`). The per-car list I decoded earlier
(`VEHICLES/VEHICLELIST.BNDL`, 112-byte records keyed by vehicle ID) is this table. So the car's sound set is the `Car Select Sound`
field of its `VehicleListEntry`, and the role value is read from that record. Next: locate the field offset of `Car Select Sound`
within the entry, and check it against the role-2 value.

**Field offset of `Car Select Sound` (read, not yet found).** No inflated stream of `VEHICLES/VEHICLELIST.BNDL` contains the name
`VehicleListEntry` or any of its field names as text, so the schema that gives the field order is keyed by hash or lives in
another bundle. The field offset of `Car Select Sound` within the 112-byte record therefore cannot be read from the strings. The
next step is to derive the field-name hashes of the `VehicleListEntry` fields (the `vlt_hash` and the base-40 forms already
tested) and match them against the record's key words.

**`VehicleListEntry` field hashes (read, negative).** The base-40 12-character hash from `NFS13.exe` (`0x009ED220`) was computed for
the field names `Vehicle ID`, `Vehicle Name`, `Vehicle Short Name`, `Vehicle Tier`, `Car Select Sound`, `CarSelectSound`,
`Vehicle Power`, `Vehicle Year`, `Vehicle Power RPM`, `Livery Vehicle ID`, `Vehicle Image`, `VehicleSound`, `SoundSet`,
`Sound Set` and `VehicleSoundSet` (low and high halves). None appears among the 32-bit words of the 70-record per-car table
(stream `0xF76`). So the field keys are not that hash of the field names.

**`VehicleListEntry` field offsets (read).** The registration table at `0x42ADF2`..`0x42B0xx` in `NFS13.exe` registers each field
with its byte offset in the record. The record is `0x70` (112) bytes, as in the list. The field `Car Select Sound` (name at
`0xF9B3F0`, alias `CarSelectSound` at `0xF9B404`) has offset **`0x28`** (byte 40), type `0xF`, count 1. The field `Vehicle Tier`
has offset `0x68` (byte 104), and `Vehicle Power` (or the preceding field) has `0x6A`, `0x6C`.

Checked across the 70 per-car records (stream `0xF76`): byte 40 (`Car Select Sound`) is **0 for 66 cars**, and the other four hold
`14691`, `14692`, `14693` and `14725`. So the sound reference is sparse, most cars use none, and the four non-zero values are the
car-select sound references. Byte 100 (word 25) remains the categorical field (values 18, 8, 0, 1, 16), so word 25 is not the
`Car Select Sound` field. The role-2 value is still not located in these records.

**`VehicleListEntry` field table (read, complete).** The registration calls at `0x42AE27`..`0x42B127` in `NFS13.exe` name each field
and its byte offset: `Vehicle ID`, `Livery Vehicle ID`, `Vehicle Name`, `GamePack ID` (`0x2C`), `Vehicle Description` (`0x0C`),
`Vehicle Name Image` (`0x24`), `Vehicle Short Name` (`0x10`), `Vehicle Image` (`0x1C`), `Online Selection Image` (`0x20`),
`PaintPackGroup` (`0x34`), `TopSpeed` (`0x38`), `Acceleration` (`0x40`), `Power` (`0x44`), `PowerRPM` (`0x6A`), `Year` (`0x6C`),
`Tier` (`0x68`), `CarSelectSound` (`0x28`), `ManufacturerId` (`0x14`), and the rollout fields (`0x48` to `0x60`). The record size is `0x70`.

None of these fields is named as a sound-owner role. The only sound reference in the entry is `CarSelectSound` at `0x28`, which is
zero for 66 of the 70 cars. The role-2 value is therefore not a field of the entry itself. It should sit in the sound set that
`CarSelectSound` points to, and the four non-zero references (`14691`, `14692`, `14693`, `14725`) are the candidate sets to check.

**`CarSelectSound` target values (read, negative for role 2).** The four non-zero `CarSelectSound` values (`14691`, `14692`,
`14693`, `14725`) occur 348 times across the inflated `VEHICLES/` and `SOUND/` streams (`14691` 222 times, `14725` 74, `14693` 35,
`14692` 17), including in the per-car list and in vehicle bundles such as `VEH_1085007_HI`. None of the occurrences has a raw `2`
within 16 bytes. So these references do not put a role value of 2 beside them, and the role-2 value is not in the sound-set
references in this form.

**Definition search for the `14691`-range values (read, negative).** The values `14691`, `14692`, `14693` and `14725` were searched in
every inflated `VEHICLES/` and `SOUND/` stream for an occurrence within 32 bytes of a Genesys object key (`0x2EDB25E7`,
`0xB92285F2`, `0x0B3FE48A`, `0xA32A93D6`, `0x8A89E1A3`). **Zero** definitions were found, so these values are not stored
beside the sound-set schema keys. They are references to a table whose entries are not in these streams.

**Remaining bundle sets (read, weak lead).** The `14691`-range values were searched in the inflated streams of the other 46 bundles
(`GAMELOGIC/**`, the root `*.BNDL` and `*.BIN`). Seven occurrences were found, all in `GLOBALEFFECTS.BNDL` (`14691` five times, `14692`
twice). The effects bundle is not a sound-set table, so these are most likely coincidental values. The table that defines the
`CarSelectSound` targets is still not located.

**`NFS13.exe` sound-set loader, step (read).** The string `_VEHICLESOUND` (VA `0xFAD464`) is used at `0x57BE31`. The routine near
`0x57BE00` builds a resource name: it copies a prefix (`_VEHICLESOUND` or the alternative at `0xFAD474`) and appends the caller's
string with `0x00EDA4D0`, then calls `0x00ACEEA0` with the result. So this is the vehicle-sound resource-name builder, the entry
point to the per-vehicle sound-set loader. Next: read `0x00ACEEA0` to see the resource it opens, and whether that resource holds the
`CarSelectSound` targets.

**`NFS13.exe` sound-set loader, step (read).** The resource-name builder `0x57BE00` passes its result to `0x00ACEEA0`, a thin
wrapper that calls the resource opener `0x00A1D640` with the same argument (`0x00ACEEB0` wraps `0x00A1D5C0` the same way). So the
chain is: `_VEHICLESOUND` name builder, then `0xACEEA0`, then `0xA1D640`. Next: read `0x00A1D640` to see which file it opens, and
whether that file holds the `CarSelectSound` targets.

**`NFS13.exe` sound-set loader, step (read): `0x00A1D640` is a case-insensitive CRC-32.** The routine folds uppercase ASCII to
lowercase (`0x41`..`0x5A` gain `0x20`) and runs the table-driven reflected CRC-32 (table at `0x010D0848`). The loader therefore finds
a sound set by the CRC-32 of its lowercased name, not by a file path. The `CarSelectSound` targets (`14691`, `14692`, `14693`,
`14725`) are then indexes that a CRC-32 lookup maps to. Next: list candidate lowercased set names, compute their CRC-32, and match
the results against the stored keys.

**CRC-32 key match (read, negative).** The case-insensitive CRC-32 of 45 candidate sound-set names (the car names `bmwm3`, `bmwm3gtr`,
`copheli`, `coplightblue`, `coplightred`, `coplightwhite`, `corvette`, `mustanggt`, `porsche911`, each alone and with the prefixes
`_vehiclesound_`, `vehiclesound_`, `_vehiclesound` and `vehiclesound`) was searched for in every inflated `VEHICLES/` and `SOUND/`
stream. **Zero hits.** The sound-set names are therefore not these car names, or the keys are stored in a form I have not found.

**Source of the sound-set names (read, not found).** Three places were searched for the names that the CRC-32 lookup needs:
- `NFS13.exe`: no lowercase sound or car set names (a lowercase search of sound and car words returned nothing).
- `VEHICLES/VEHICLELIST.BNDL` inflated streams: 49 readable strings, all Genesys or nitrous and upgrade terms; no car names.
- `GDFBinary_en_US.dll`: none of the car display names (`M3 GTR`, `Mustang`, `Corvette`, `Porsche`, `BMW`, `Traffic`) in ASCII or
  UTF-16. The word `Cop` appears twice in UTF-16.
So the sound-set names are not stored as readable text in any file searched. Their source is not located.

**Sound-set names in the running `speed.exe` (read, negative at the menu).** Cheat Engine searched the attached process for `vehiclesound`
and `_VEHICLESOUND` (case as the loader builds them). Both return zero addresses at the menu. The loader builds the lowercased set names
when a car loads, so this search needs a race running to be conclusive.

**Numeric sound-set name (read).** The name builder near `0x57BDxx` formats a number with the `%d`/`%u`/`%lld` conversion
(`0x00427F80`) and prefixes `_VEHICLESOUND`. So a sound-set name is the prefix plus a decimal number, and its key is the
case-insensitive CRC-32. Four prefixes (`_vehiclesound`, `vehiclesound`, `_vehiclesound_`, `vehiclesound_`) with numbers 0 to 29999
were tested against the inflated `VEHICLES/` and `SOUND/` streams. That gives 3,807 hits, from many numbers. **But the four
`CarSelectSound` targets (`14691`, `14692`, `14693`, `14725`) give zero hits** under all four prefixes. So the `CarSelectSound`
values are not CRC-32 keys of these names: they are IDs, and their sound sets are keyed differently.

**Function that builds the sound-set name (read).** The enclosing function of the `_VEHICLESOUND` name builder has a `sub esp, 0x338`
prologue at `0x0057BC70` (the bytes before it are padding). It has no direct `call` callers, so it is reached through a virtual slot
or a function pointer. Its `CarSelectSound` ID argument arrives on the stack (`[esp+0x35C]` at the name builder). The next step is to
find the vtable or table that holds `0x0057BC70` and the ID source for that call.

**The owner of the sound-set name builder (read).** The address `0x0057BC70` is stored once, at `0x00F9A3C4`, which is slot 0 of
a vtable at `0x00F9A3C4`. The constructor at `0x00425360` installs that vtable (`mov [eax], 0xF9A3C4`), together with sub-object
vtables at `+0x18` (`0xF99700`), `+0x28` (`0xF98EF8`), `+0x50` (`0xF98DB0`) and `+0x98` (`0xF98DAC`), and clears a block of 16 pairs from
`+0x428`. So the name builder `0x0057BC70` is the first virtual method of a large object that owns the sound-set lookup. Next: read that
object's other methods, and find which one passes the `CarSelectSound` ID.

**Sound-object vtable (read).** The vtable at `0x00F9A3C4` has 18 method slots: slot 0 `0x0057BC70` (the name builder), 1 `0x00573F70`,
2 `0x00531A70`, 3 `0x00531A80`, 4 `0x00531A90`, 5 `0x005719C0`, 6 `0x005719D0`, 7 `0x00531AC0`, 8 `0x00531AD0`, 9 `0x00531AE0`,
10 `0x00425620`, 11 `0x00A874D0`, 12 `0x00A81F90`, 13 `0x00A8CC20`, 14 `0x00425600`, 15 `0x00425610`, 16 `0x00A8B180`, 17 `0x00A8B200`.
Slot 18 onward is text, so the vtable ends at slot 17. The callers of slot 0 are indirect (`call [reg]`), and they are not yet identified.

**Classifying the indirect callers of slot 0 (read, method unreliable).** A byte scan for indirect calls (`FF 10`, `11`, `12`, `13`, `16`,
`17`) within 0x100 bytes of a `[reg+0x28]` load found 23 candidate sites. The four in the sound-code range (`0x570B9F`, `0x578158`,
`0x57EE0C`, `0x57FC01`) disassemble from mid-instruction offsets, so they are false matches, not real calls. A reliable classification
needs instruction-boundary disassembly of the whole binary, which this scan does not provide. The caller of slot 0 is still unidentified.

**Full-binary indirect-call pass (read).** A linear disassembly of the whole code section (instruction boundaries real) finds only eight
plain indirect calls of the form `call dword ptr [reg]`. Most of them are in the `0x00F0F7xx`..`0x00F1873A` region. The one that fits a
vtable slot-0 call is at `0x00F0FAB3`: it follows `mov eax, [ecx]` and calls `[eax]`, which is slot 0 when `ecx` is the sound object. This is
a **candidate, not verified**: the owner of `ecx` at `0x00F0FAB3` is not yet confirmed to be the object at `0x00F9A3C4`.

**The candidate slot-0 caller is a generic size combiner (read, weak).** The function containing `0x00F0FAB3` starts at `0x00F0FAA0`. It
caches its result at `[esi+0xC]` (computed only when that is negative) and sums slot-0 results of two child objects, `[esi+8]` and
`[esi+4]`: `call [eax]` on the first child, then on the second. It passes no `CarSelectSound` ID. So it is a generic composite size
calculation, and it calls slot 0 of whatever its children are. The candidate is **rejected** as the name-builder caller unless one of
its children is the sound object at `0x00F9A3C4`, which is not established. The slot-0 caller remains unidentified.


**The candidate slot-0 caller is a generic size combiner (read, weak).** The function containing `0x00F0FAB3` starts at `0x00F0FAA0`. It
caches its result at `[esi+0xC]` (computed only when that is negative) and sums slot-0 results of two child objects, `[esi+8]` and
`[esi+4]`. It passes no `CarSelectSound` ID. So it is a generic composite size calculation. The candidate is rejected as the name-builder
caller unless one of its children is the sound object at `0x00F9A3C4`, which is not established. The slot-0 caller remains unidentified.

**The size combiner's class (read).** The combiner at `0x00F0FA9E` (entry, a hot-patch `mov edi, edi` before `0x00F0FAA0`) is slot 0 of
the vtable at `0x0109D834`. The constructor at `0x00F0FA80` installs that vtable and stores two child pointers, at `+0x4` and `+0x8`, from
its stack arguments. The vtable has slots 0 to 5 as code: `0x00F0FA9E` (size combiner), `0x00F0FAC9`, `0x00F0FAE5`, `0x00F0FF42`,
`0x00F0FF46`, `0x00F0FF58`. So this is a binary composite node. It has no direct callers, so its instances come from a factory or a registry.
The children are not identified, and the sound object at `0x00F9A3C4` is a different class.

**The composite node factory and its callers (read).** The composite nodes are created in two places that store the vtable `0x0109D834`:
the constructor `0x00F0FA80` and the chain builder `0x00F0FD86`. The chain builder (entry `0x00F0FD86`) allocates a 16-byte node from pool
`0x011BBCA4`, sets its first child to the object's current head (`[esi]`), sets its second child to its argument, and stores the node as the
new head. Five callers (`0x00F10010`, `0x00F10334`, `0x00F1039A`, `0x00F105A9`, `0x00F1060C`) use it. The first is an `Add(value)` method at
`0x00F0FFE2`: for a value above zero, with state byte `[esi+4] <= 1`, it makes a leaf with `0x00F0F9F7`, then links it with the chain builder.
Values 2 and 3 take a separate path (`0x00F0FE1B`). So the composite is a list of small codes, one per added value, and the codes come from the
callers. The next step is to identify the caller that adds the `CarSelectSound` ID.

**The other chain-builder callers (read).** Of the four remaining callers of `0x00F0FD86`, two are real call sites:
- `0x00F10334` links a child node copied from another node (`[eax+4]`).
- `0x00F1039A` links a leaf made by `0x00F0F94A` (the value-to-leaf factory).
The callers at `0x00F105A9` and `0x00F1060C` disassemble from mid-instruction offsets, so they are not calls. The `CarSelectSound` ID enters
the list as a leaf value through `0x00F0F94A`, so the next step is the caller of `0x00F0F94A` that supplies the ID.

**Callers of the leaf factory and of Add (read).** The value-to-leaf factory `0x00F0F94A` has three callers: `0x00F0FCB4` and `0x00F0FE01`
(both inside the list code) and `0x00F1038E` (the real caller noted above). The `Add(value)` method `0x00F0FFE2` has nine callers
(`0x00F102F1`, `0x00F1031B`, `0x00F10372`, `0x00F10714`, `0x00F10CF6`, `0x00F10F2D`, `0x00F11116`, `0x00F1246D`, `0x00F13DDD`). The first,
at `0x00F102F1`, belongs to a copy-and-add function at `0x00F102D9`: it copies a composite's two children (`[ecx]`, `[ecx+4]`) into the new
object, then calls `Add` with its own argument (`[ebp+0xC]`). So the value that carries the `CarSelectSound` ID enters at the caller of
`0x00F102D9`, one level further up. That caller is the next step.

**Callers of the copy-and-add function `0x00F102D9` (read).** Three callers: `0x00F10BB3` (a window that disassembles from mid-instruction),
`0x00F10F65` and `0x00F11B79`.
- `0x00F10F65` builds its value from the literal `0x5B` (91) through `0x00F100A7`: a fixed code, not data-driven.
- `0x00F11B79` pushes its own function argument (`[ebp+8]`) with a flag `1`, so it is the data-driven path. Its argument is the value that
  carries the list entry, and the `CarSelectSound` ID is expected to arrive there.
The next step is the function that contains `0x00F11B79`, and its callers, to find where the `[ebp+8]` argument comes from.

**The enclosing function is a data parser (read).** The function containing the data-driven call `0x00F11B79` starts at `0x00F11862`
(`mov edi, edi; push ebp; mov ebp, esp`). It has four direct callers: `0x00F11CDE`, `0x00F11EBE`, `0x00F123D8` and `0x00F1350E`. Two of
them show what it is fed. `0x00F1350E` passes a pair of stack buffers and a counter (`ebx`). `0x00F123D8` passes its own argument
(`[ebp+8]`) and stores a global at `0x011BBCC4`. So the function parses values out of a data buffer, passing the buffer down a chain of
callers. The `CarSelectSound` IDs are read by this parser from the entry's data, so the ID-to-sound-set mapping is a parse of the list
records, not a separate table. The remaining step is to identify the record format that this parser reads.

**The record format is a text expression (read).** The parser `0x00F11862` reads one character at a time from a global cursor at
`0x011BBCC4`. It dispatches on the character: `'A'` (`0x41`) and the characters `0x2F` to `0x39` (`/` and the digits) are handled by
separate branches, with digit codes resolved through a table at `0x0109D5E8`, and a fallback branch at `0x00F11CFC`. It stores a
two-word result (a value and a flag) at the caller's output (`[ebp+8]`). So the list entries are small text expressions over digits and
operators, parsed into node values. The ID-to-sound-set mapping is therefore a rule in that text, and its source string is the next thing to find.

**The parser's text cursor (read).** The cursor global is `0x011BBCC4` (bytes `c4 bc 1b 01`; an earlier search used a wrong byte and found none).
It is written at about 28 sites (`0x00F0F358` to `0x00F1406D`) and read at about 70 sites, because the parser advances it. The number
reader at `0x00F0F340` shows the pattern: it increments the cursor, reads a character, and parses digits with `0x00F0F156`. So the cursor
walks a text buffer. Its initial value is set by an outer caller that loads a string pointer; that root setter is not yet located.

**Root setter search (read, not yet found).** The cursor writes at `0x00F131A2`, `0x00F136C0` are internal increments of the parser.
The special-form parser at `0x00F123A9` reads the cursor and dispatches on the markers `?` (`0x3F`) and `$` (`0x24`). Its only direct
callers, `0x00F11DB9` and `0x00F131D2`, are inside the parser itself. So the top-level entry that first loads a string pointer into the
cursor is above these. The root setter is not yet located.

**Walking up from the parser (read).** The function that contains the stack-buffer caller `0x00F1350E` starts at `0x00F133B7`. Its
direct callers are `0x00F10748`, `0x00F1109C`, `0x00F12200`, `0x00F13229`, `0x00F1327E`, `0x00F13781`, `0x00F13D76` and `0x00F13DBE`. All of
them are inside the expression-parser region (`0x00F10000`..`0x00F14000`), so the walk has not yet left the parser. The top-level entry is
one of these callers' callers, further up the same region. Not yet located.

**The expression region is closed under static calls (read).** The enclosing functions of the eight callers are `0x00F10720`,
`0x00F11001`, `0x00F121A5`, `0x00F13149`, `0x00F1366E` and `0x00F13A4D`. Their callers are all inside the region (`0x00F0F000`..`0x00F15000`),
and none of the entries is stored as a function pointer. So the expression parser is entered only through an indirect route that static
reading cannot follow. The top-level text source must be found at run time: a data breakpoint on the cursor `0x011BBCC4` would show the
string the parser starts from, once a race loads.
