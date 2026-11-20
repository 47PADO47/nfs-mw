# handling-probe

A fixture mod for the `bb.handling` host seam ([docs/modding/api-handling.md](../../docs/modding/api-handling.md)). It is not
a handling mod to play with: it exists so the host tests can load a real library, call a real tyre hook and check what the
host does with the answers. It lives outside the game's workspace, like [`hello`](../hello), so a plain build never
compiles it.

```sh
cargo build --manifest-path mods/handling-probe/Cargo.toml
cargo test  --manifest-path mods/handling-probe/Cargo.toml     # the pure models
cargo test -p blackbox-modding --test handling_probe           # builds it and loads it with the real loader
```

The model is chosen by `model = "..."` in the mod's `config.toml` (read through `bb.config`; `linear` without one):

| `model` | What the hook does | What the host does |
|---|---|---|
| `linear` | force proportional to slip, friction limit per axis, wheel spin integrated with the given torques | the car drives on it |
| `friction-circle` | constant friction coefficient on the combined slip | the car drives on it, differently |
| `not-handled` | declines every wheel | the built-in tyres run, no fault |
| `panics` | panics on every call | the SDK returns an error, a fault is counted, the fifth quarantines the hook |
| `nan` | answers a force that is not a number | the vehicle refuses it, a fault is counted, the fifth quarantines the hook |

Every variant declines a vehicle that is not the player's (`Role::Player`). With `bb.vehicles` 1.3 the probe reads the car's
`spec` when the `vehicle_revision` of a query changes and logs a line.

The two working models are written to show that the hook does not assume a model: they share the hook and the wheel-spin
integration helper, and nothing else.
