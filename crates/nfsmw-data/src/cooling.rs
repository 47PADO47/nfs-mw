//! The paint cooling table of the attribute database: the `fecooling` class has one collection of eleven floats, the
//! factor the career car's heat is multiplied by when the player fits a visual part. Spec:
//! `docs/specs/career-cooling.md`.

use blackbox_attrib::Database;

/// The class that holds the table.
pub const CLASS: &str = "fecooling";

/// The factor of each part kind (`NewBodyKit` ... `NewWindowTint`). 1.0 leaves the heat as it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeCooling {
    pub body_kit: f32,
    pub decal: f32,
    pub hood: f32,
    pub numbers: f32,
    pub paint: f32,
    pub rim: f32,
    pub rim_paint: f32,
    pub roof_scoop: f32,
    pub spoiler: f32,
    pub vinyl: f32,
    pub window_tint: f32,
}

impl FeCooling {
    /// The factors as `(field name, value)` in the order of the class's fields (alphabetical).
    pub fn entries(&self) -> [(&'static str, f32); 11] {
        [
            ("NewBodyKit", self.body_kit),
            ("NewDecal", self.decal),
            ("NewHood", self.hood),
            ("NewNumbers", self.numbers),
            ("NewPaint", self.paint),
            ("NewRim", self.rim),
            ("NewRimPaint", self.rim_paint),
            ("NewRoofScoop", self.roof_scoop),
            ("NewSpoiler", self.spoiler),
            ("NewVinyl", self.vinyl),
            ("NewWindowTint", self.window_tint),
        ]
    }
}

/// The cooling table: the `default` collection of `fecooling` (the class has no other), or its only collection.
/// `None` when the class is missing or any of the eleven fields is not a number.
pub fn fe_cooling(db: &Database) -> Option<FeCooling> {
    let row = db.collection(CLASS, "default").or_else(|| db.collections_of(CLASS).next())?;
    Some(FeCooling {
        body_kit: row.get_f32("NewBodyKit")?,
        decal: row.get_f32("NewDecal")?,
        hood: row.get_f32("NewHood")?,
        numbers: row.get_f32("NewNumbers")?,
        paint: row.get_f32("NewPaint")?,
        rim: row.get_f32("NewRim")?,
        rim_paint: row.get_f32("NewRimPaint")?,
        roof_scoop: row.get_f32("NewRoofScoop")?,
        spoiler: row.get_f32("NewSpoiler")?,
        vinyl: row.get_f32("NewVinyl")?,
        window_tint: row.get_f32("NewWindowTint")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The install's table (read in this pass): eleven factors between 0.69 (body kit) and 0.99 (window tint).
    /// Skipped without `NFSMW_GAME_DIR`.
    #[test]
    #[ignore = "needs the installed game"]
    fn the_installed_cooling_table_reads_eleven_factors() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = game_install::GameDir::open(dir).unwrap();
        let db = crate::shared::attributes(&dir).unwrap();
        let table = fe_cooling(&db).expect("the install has a fecooling collection");
        let expected = [0.69, 0.98, 0.9, 0.97, 0.7, 0.95, 0.97, 0.97, 0.9, 0.75, 0.99];
        for ((name, value), want) in table.entries().into_iter().zip(expected) {
            assert!((value - want).abs() < 1e-6, "{name} = {value}, the install holds {want}");
        }
    }
}
