//! The clip sets of the rider animation blueprint, one per bike type (`docs/specs/ride6-rider-styles.md`).
//!
//! `SET Animations` of the blueprint switches on an integer `BikeType` (0 race, 1 naked, 2 motard, 3 scooter, 4 enduro, 5
//! bagger) and, inside it, on `RidingStyle` (0 to 2), and assigns the clip variables the pose is built from (`Leaning_Standard_*`,
//! `Leaning_Aero_*`, `Standard_Pose`, `AeroPose_*`, `Wheelie_*`, `Stoppie`, `AdditiveShake_*`, `Wheelie_Standing_BS`, `Drift_*`,
//! `Retro_*`). Each table below is the result of that function for one type; the names are the assets of the install
//! (`riderShared/Animations/<folder>/`). A clip a type does not have is `None` (race and naked bikes have no drift pose, a
//! scooter has no standing wheelie).

/// Which of the blueprint's three riding styles a lean set belongs to: the clips of a style are chosen by `RidingStyle`.
pub const STYLES: usize = 3;

/// The lean clips of one riding style: left and right at low speed (`Leaning_Standard_*`) and at high speed (`Leaning_Aero_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lean {
    pub low: [&'static str; 2],
    pub high: [&'static str; 2],
}

/// The static centre poses the lean blends away from (`Standard_Pose`, `AeroPose_Low`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Centre {
    pub low: &'static str,
    pub high: &'static str,
}

/// The reverse walk: the clip that starts it and the one that loops (`Retro_Start_BothFeet`, `Retro_Loop_R`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Walk {
    pub start: &'static str,
    pub looped: &'static str,
}

/// Everything the graph needs of one bike type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipSet {
    /// The type's name as the files carry it (`contacts.json` `style.name`, the graph's file name).
    pub name: &'static str,
    /// The blueprint's `BikeType` value.
    pub bike_type: u8,
    /// The lean clips of the three riding styles, by `RidingStyle`.
    pub lean: [Lean; STYLES],
    /// The centre poses of the three styles. A style whose own pose is missing from the pack uses the first entry.
    pub centre: [Centre; STYLES],
    /// `Wheelie_L_Min`, `Wheelie_L_Max`, `Wheelie_R_Min`, `Wheelie_R_Max`.
    pub wheelie: [&'static str; 4],
    pub stoppie: &'static str,
    /// `AdditiveShake_LowSpeed`, `AdditiveShake_HighSpeed`.
    pub shake: [&'static str; 2],
    /// The standing wheelie: `Standing`, `Standing_L`, `Standing_R`, `StandingInterpolation` of the blend space
    /// `Wheelie_Standing_BS`.
    pub standing: Option<[&'static str; 4]>,
    /// `Drift_L`, `Drift_R`.
    pub drift: Option<[&'static str; 2]>,
    pub walk: Walk,
}

const fn lean(low: [&'static str; 2], high: [&'static str; 2]) -> Lean {
    Lean { low, high }
}

const fn centre(low: &'static str, high: &'static str) -> Centre {
    Centre { low, high }
}

/// Bike type 0: sport and racing bikes. The three styles are the three rider personalities of the data (Rossi, Lorenzo,
/// Schwantz) with the body hung off the bike.
pub const RACE: ClipSet = ClipSet {
    name: "race",
    bike_type: 0,
    lean: [
        lean(
            ["race_Rossi_standard_Left", "race_Rossi_standard_Right"],
            ["race_Rossi_aero_Left", "race_Rossi_aero_Right"],
        ),
        lean(
            ["race_Lorenzo_standard_Left", "race_Lorenzo_standard_Right"],
            ["race_Lorenzo_aero_Left", "race_Lorenzo_aero_Right"],
        ),
        lean(
            ["race_Schwantz_standard_Left", "race_Schwantz_standard_Right"],
            ["race_Schwantz_aero_Left", "race_Schwantz_aero_Right"],
        ),
    ],
    centre: [centre("race_standard_pose", "race_aeroPose_Down"); STYLES],
    wheelie: ["race_Wheelie_L_min", "race_Wheelie_L_max", "race_Wheelie_R_min", "race_Wheelie_R_max"],
    stoppie: "race_stoppie",
    shake: ["race_Shake_LowSpeed", "race_Shake_HighSpeed"],
    standing: Some([
        "race_wheelie_Standing_anim",
        "race_wheelie_Standing_anim_L",
        "race_wheelie_Standing_anim_R",
        "race_wheelie_StandingInterpolation_anim",
    ]),
    drift: None,
    walk: Walk { start: "race_retro", looped: "race_retro_loop" },
};

/// Bike types 1 and 5: naked bikes and baggers (the blueprint sends both to the same clips).
pub const NAKED: ClipSet = ClipSet {
    name: "naked",
    bike_type: 1,
    lean: [
        lean(
            ["naked_Balanced_Standard_Left", "naked_Balanced_Standard_Right"],
            ["naked_Balanced_aero_Left", "naked_Balanced_aero_Right"],
        ),
        lean(
            ["naked_ShoulderOut_standard_Left", "naked_ShoulderOut_standard_Right"],
            ["naked_ShoulderOut_aero_Left", "naked_ShoulderOut_aero_Right"],
        ),
        lean(
            ["naked_Old_School_Standard_Left", "naked_Old_School_Standard_Right"],
            ["naked_Old_School_Aero_Left", "naked_Old_School_Aero_Right"],
        ),
    ],
    centre: [centre("naked_low_standard_pose", "naked_aeroPose_Down"); STYLES],
    wheelie: ["naked_wheelie_min_L", "naked_wheelie_max_L", "naked_wheelie_min_R", "naked_wheelie_max_R"],
    stoppie: "naked_stoppie",
    shake: ["naked_Shake_LowSpeed", "naked_Shake_HighSpeed"],
    standing: Some([
        "naked_wheelie_Standing_anim",
        "naked_wheelie_Standing_anim_L",
        "naked_wheelie_Standing_anim_R",
        "naked_wheelie_StandingInterpolation_anim",
    ]),
    drift: None,
    walk: Walk { start: "naked_retro", looped: "naked_retro_loop" },
};

/// Bike type 2: supermotos.
pub const MOTARD: ClipSet = ClipSet {
    name: "motard",
    bike_type: 2,
    lean: [
        lean(
            ["motard_Leaning_Knee_LowSpeed_L", "motard_Leaning_Knee_LowSpeed_R"],
            ["motard_Leaning_Knee_HighSpeed_L", "motard_Leaning_Knee_HighSpeed_R"],
        ),
        lean(
            ["motard_Leaning_LowSpeed_L", "motard_Leaning_LowSpeed_R"],
            ["motard_Leaning_HighSpeed_L", "motard_Leaning_HighSpeed_R"],
        ),
        lean(
            ["motard_Old_School_standard_L", "motard_Old_School_standard_R"],
            ["motard_Old_School_aero_L", "motard_Old_School_aero_R"],
        ),
    ],
    centre: [
        centre("motard_CenterPose_LowSpeed", "motard_CenterPose_HighSpeed"),
        centre("motard_CenterPose_LowSpeed", "motard_CenterPose_HighSpeed"),
        centre("motard_old_school_standard_pose", "motard_aero_pose"),
    ],
    wheelie: ["motard_wheelie_min_L", "motard_wheelie_max_L", "motard_wheelie_min_R", "motard_wheelie_max_R"],
    stoppie: "motard_stoppie",
    shake: ["motard_Shake_LowSpeed", "motard_Shake_HighSpeed"],
    standing: Some([
        "motard_wheelie_Standing_anim",
        "motard_wheelie_Standing_anim_L",
        "motard_wheelie_Standing_anim_R",
        "motard_wheelie_StandingInterpolation_anim",
    ]),
    drift: Some(["motard_drift_pose_L", "motard_drift_pose_R"]),
    walk: Walk { start: "motard_retro_both_feet", looped: "motard_retro_loop_R" },
};

/// Bike type 3: scooters (the on-road lean set of `TrackCategoryAnimationModifier` 0).
pub const SCOOTER: ClipSet = ClipSet {
    name: "scooter",
    bike_type: 3,
    lean: [lean(
        ["rider_scooter_Leaning_standard_L", "rider_scooter_Leaning_standard_R"],
        ["rider_scooter_Leaning_aero_L", "rider_scooter_Leaning_aero_R"],
    ); STYLES],
    centre: [centre("rider_scooter_standard_pose", "rider_scooter_aero_pose"); STYLES],
    wheelie: [
        "rider_scooter_wheelie_min_L",
        "rider_scooter_wheelie_max_L",
        "rider_scooter_wheelie_min_R",
        "rider_scooter_wheelie_max_R",
    ],
    stoppie: "rider_scooter_stoppie",
    shake: ["rider_scooter_Shake_LowSpeed", "rider_scooter_Shake_HighSpeed"],
    standing: None,
    drift: Some(["rider_scooter_drift_pose_L", "rider_scooter_drift_pose_R"]),
    walk: Walk { start: "rider_scooter_retro", looped: "rider_scooter_retro_loop_R" },
};

/// Bike type 4: enduro and adventure bikes. The lean and wheelie clips are the motard's; the poses, the drift and the walk
/// are the enduro's own.
pub const ENDURO: ClipSet = ClipSet {
    name: "enduro",
    bike_type: 4,
    lean: [
        MOTARD.lean[0],
        MOTARD.lean[1],
        lean(
            ["enduro_Old_School_Standard_L", "enduro_Old_School_Standard_R"],
            ["enduro_Old_School_aero_L", "enduro_Old_School_aero_R"],
        ),
    ],
    centre: [centre("enduro_standard_pose", "enduro_aeroPose"); STYLES],
    wheelie: MOTARD.wheelie,
    stoppie: MOTARD.stoppie,
    shake: MOTARD.shake,
    standing: MOTARD.standing,
    drift: Some(["enduro_drift_pose_L", "enduro_drift_pose_R"]),
    walk: Walk { start: "enduro_retro_both_feet", looped: "enduro_retro_loop_R" },
};

/// Every set, in the order of the blueprint's `BikeType`.
pub const SETS: [&ClipSet; 5] = [&RACE, &NAKED, &MOTARD, &SCOOTER, &ENDURO];

/// The set of a type's name (`race`, `naked`, `motard`, `scooter`, `enduro`).
pub fn by_name(name: &str) -> Option<&'static ClipSet> {
    SETS.iter().copied().find(|set| set.name == name)
}

/// Every clip name the set uses, for a check of the export against the tables.
pub fn clip_names(set: &ClipSet) -> Vec<&'static str> {
    let mut out = Vec::new();
    for style in 0..STYLES {
        out.extend(set.lean[style].low);
        out.extend(set.lean[style].high);
        out.push(set.centre[style].low);
        out.push(set.centre[style].high);
    }
    out.extend(set.wheelie);
    out.push(set.stoppie);
    out.extend(set.shake);
    out.extend(set.standing.into_iter().flatten());
    out.extend(set.drift.into_iter().flatten());
    out.push(set.walk.start);
    out.push(set.walk.looped);
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_set_is_found_by_its_name() {
        for set in SETS {
            assert_eq!(by_name(set.name).map(|s| s.bike_type), Some(set.bike_type));
        }
        assert!(by_name("cruiser").is_none());
    }

    #[test]
    fn the_enduro_set_borrows_the_motard_lean_and_wheelie_clips() {
        assert_eq!(ENDURO.lean[0], MOTARD.lean[0]);
        assert_eq!(ENDURO.wheelie, MOTARD.wheelie);
        assert_ne!(ENDURO.walk, MOTARD.walk);
    }

    #[test]
    fn only_the_scooter_has_no_standing_wheelie_and_race_and_naked_have_no_drift() {
        assert!(SCOOTER.standing.is_none());
        assert!(RACE.drift.is_none() && NAKED.drift.is_none());
        assert!(MOTARD.drift.is_some() && ENDURO.drift.is_some() && SCOOTER.drift.is_some());
    }
}
