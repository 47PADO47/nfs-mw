# RIDE 6 bike customisation and upgrades

Code: [`tools/ride6-exporter/{upgrades,option_export,upgrade_tuning}.py`](../../tools/ride6-exporter) and
[`mods/ride6-bikes/src/{options,options_pack,options_manifest,build_pieces,shop}.rs`](../../mods/ride6-bikes/src).

- **Pages:** [modding/ride6-bike-upgrades.md](../modding/ride6-bike-upgrades.md), [formats/ride6-bike-parts.md](../formats/ride6-bike-parts.md#customisation-data-and-upgrades).
- **Sources read:** the user's own RIDE 6 install (UE 5.6.1 build `5.6.1-1697004`), unencrypted IoStore packages read with
  CUE4Parse and the game's `.usmap`: `Content/assets/graphics/Bindings/Bike/*` (the tables), the data assets they name under
  `graphics/vehicles/bikes/`, `Content/assets/ui/bitmaps/datatables/DTVehicleParts`, `Content/assets/data/datatables/BikeUpgrades/BikePhysicDataTable`
  and `PhysicsRide6/.../TuningComponent/TuningComponentData_Upgrade*` with the resolved physics dump. No decompiled code, no
  binary was read, no game data is in the repository.
- **Facts from the data:** the row keys and counts, the upgrade rows' member paths, modifier types and ranges, the bikes that list
  them, the 0 to 1 entry values of the team table, the default entry value 1.
- **Inferences and choices** (each marked in the pages): the `AdaptiveMultiplier` formula; the equal steps and the number of levels
  of a ladder (3, 2, 2); which ladder carries `UPGRADE_Mass` (the chassis); the fit rule for rims (3 percent, 0.75 to 1.35); the
  bar pads offered only to bikes with a stock pad; the sizes and levels of the part packs.
- **Not found:** what an engine sub-kit, the suspension kit or the transmission kit change; prices; a map from a kit to an
  upgrade value for a road bike; which bike takes which aftermarket rim.
