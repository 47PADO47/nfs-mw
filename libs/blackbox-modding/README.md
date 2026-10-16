# blackbox-modding

Native mod plugins for Black Box games: the stable C ABI between a game and a mod library, the Rust SDK a
mod author links, and (later) the host loader. The design is in
[ADR 0005](../../docs/decisions/0005-native-mod-plugins.md); the catalogue of what a mod can call is in
[docs/modding/](../../docs/modding/README.md).

Features:

- `abi` (default): `#[repr(C)]` tables, status codes, the ABI version and the entry-symbol names. Layout is
  locked by snapshot tests.
- `sdk`: the `Mod` trait, `declare_mod!`, the panic guard, and the host-table wrappers (logging, game id,
  interface query).
- `host`: reserved for the loader.

The crate is engine-generic: no game names, paths or defaults. `unsafe` is allowed only in `abi` and
`sdk::ffi`.
