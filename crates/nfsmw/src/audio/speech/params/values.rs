//! The values the speech parameters take, as the bit each choice sets (`1 << n` for the nth choice), from the
//! game's speech interface. A parameter may name several choices at once by adding bits. Generated; a fact table.

pub mod code {
    pub const USE_10_CODE: u32 = 1;
    pub const DONT_USE_10_CODE: u32 = 2;
}

pub mod backup_type {
    pub const ALL_AVAILABLE_UNITS: u32 = 1;
    pub const AIR_SUPPORT: u32 = 2;
    pub const RHINO_UNIT: u32 = 4;
    pub const GENERIC_MORE_BACKUP: u32 = 8;
}

pub mod bailout_type {
    pub const GENERIC_LOW_INTENSITY: u32 = 1;
    pub const HEAVY_TRAFFIC: u32 = 2;
    pub const ROAD_CONDITIONS: u32 = 4;
    pub const DAMAGE_SUSTAINED: u32 = 8;
    pub const GENERIC_HIGH_INTENSITY: u32 = 16;
}

pub mod disp_backup_type {
    pub const NO_AVAILABLE_UNITS: u32 = 1;
    pub const _1_UNIT: u32 = 2;
    pub const ALL_AVAILABLE_UNITS: u32 = 4;
    pub const AIR_SUPPORT: u32 = 8;
    pub const RHINO_UNIT: u32 = 16;
    pub const GENERIC_GROUND_UNITS: u32 = 32;
    pub const SUPER_PURSUIT: u32 = 64;
}

pub mod encounter {
    pub const FIRST_ENCOUNTER: u32 = 1;
    pub const SUBSEQUENT_ENCOUNTER: u32 = 2;
}

pub mod ev_type {
    pub const MULTIPLE_UNITS_DOWN: u32 = 1;
    pub const GAS_STATION_FIRE: u32 = 2;
    pub const HELI_DOWN: u32 = 4;
}

pub mod heli_lost_visual {
    pub const IN_TUNNEL: u32 = 1;
    pub const BEHIND_BUILDING: u32 = 2;
    pub const UNDER_TREES: u32 = 4;
    pub const GENERIC: u32 = 8;
    pub const RESTRICTED_AIRSPACE: u32 = 16;
}

pub mod intensity {
    pub const NORMAL: u32 = 1;
    pub const HIGH: u32 = 2;
}

pub mod num_suspects {
    pub const ONE_SUSPECT: u32 = 1;
    pub const MULTIPLE_SUSPECTS: u32 = 2;
}

pub mod num_units {
    pub const ONE_UNIT_IN_PURSUIT: u32 = 1;
    pub const MULTIPLE_UNITS_IN_PURSUIT: u32 = 2;
}

pub mod position {
    pub const RIGHT_SIDE: u32 = 1;
    pub const LEFT_SIDE: u32 = 2;
    pub const AHEAD: u32 = 4;
    pub const BEHIND: u32 = 8;
}

pub mod pursuit_type {
    pub const GENERIC_SPEEDER: u32 = 1;
    pub const POSSIBLE_WANTED: u32 = 2;
    pub const HIT_AND_RUN: u32 = 4;
    pub const RECKLESS: u32 = 8;
    pub const UNIT_RAMMED: u32 = 16;
}

pub mod region {
    pub const COLLEGE_TOWN: u32 = 1;
    pub const COASTAL: u32 = 2;
    pub const CITY: u32 = 4;
    pub const ALPINE: u32 = 8;
}

pub mod roadblock_type {
    pub const ROADBLOCK_GENERIC: u32 = 1;
    pub const SPIKES: u32 = 2;
    pub const HELI_ROADBLOCK_DISP_ONLY: u32 = 4;
    pub const MULTIPLE_ROADBLOCKS_DISP_ONLY: u32 = 8;
}

pub mod roadblock_engage_type {
    pub const ROADBLOCK: u32 = 1;
    pub const SPIKES: u32 = 2;
}

pub mod rolling_strategy_type {
    pub const OFFSET: u32 = 1;
    pub const ROLLING_ROADBLOCK: u32 = 2;
    pub const BOX: u32 = 4;
    pub const ROLLING_CHICANE: u32 = 8;
    pub const HERDING: u32 = 16;
    pub const PIT: u32 = 32;
    pub const SMOKESCREEN: u32 = 64;
    pub const ROLLING_SPIKES: u32 = 128;
}

pub mod same_new {
    pub const SAME_STRATEGY: u32 = 1;
    pub const NEW_STRATEGY: u32 = 2;
}

pub mod self_strategy_type {
    pub const OFFSET: u32 = 1;
    pub const ROLLING_ROADBLOCK: u32 = 2;
    pub const HERDING: u32 = 4;
    pub const RAM: u32 = 8;
    pub const SIDE_RAM: u32 = 16;
    pub const PIT: u32 = 32;
    pub const SELF_PIT: u32 = 64;
    pub const ROLLING_SPIKES: u32 = 128;
    pub const SMOKESCREEN: u32 = 256;
}

pub mod world_object_type {
    pub const GENERIC: u32 = 1;
    pub const GUARDRAIL: u32 = 2;
    pub const TRAIN: u32 = 4;
    pub const SEMI: u32 = 8;
    pub const GAS_STATION: u32 = 16;
    pub const SPIKE_BELT: u32 = 32;
    pub const LOW_HEAT_COLLISION: u32 = 64;
}

pub mod yes_no {
    pub const YES_TRUE: u32 = 1;
    pub const NO_FALSE: u32 = 2;
}

pub mod speaker_battalion {
    pub const ROSEWOOD: u32 = 1;
    pub const COASTAL: u32 = 2;
    pub const CITY: u32 = 4;
    pub const ALPINE: u32 = 8;
    pub const SUPER_PURSUIT: u32 = 16;
    pub const RHINO_UNITS: u32 = 32;
    pub const AIR_SUPPORT: u32 = 64;
    pub const ALL_UNITS: u32 = 128;
    pub const DISPATCH: u32 = 256;
}

pub mod subject_battalion {
    pub const ROSEWOOD: u32 = 1;
    pub const COASTAL: u32 = 2;
    pub const CITY: u32 = 4;
    pub const ALPINE: u32 = 8;
    pub const SUPER_PURSUIT: u32 = 16;
    pub const RHINO_UNITS: u32 = 32;
    pub const AIR_SUPPORT: u32 = 64;
    pub const ALL_UNITS: u32 = 128;
}

pub mod time_since_lost {
    pub const LOST_RECENTLY: u32 = 1;
    pub const LOST_FOR_X_TIME: u32 = 2;
}

pub mod self_other {
    pub const CONTEXT_ABOUT_SELF: u32 = 1;
    pub const CONTEXT_ABOUT_OTHER_UNIT: u32 = 2;
}

pub mod heli_bailout_type {
    pub const FLIGHT_CONDITIONS: u32 = 1;
    pub const LOW_AMMO: u32 = 2;
    pub const FUEL_LOW: u32 = 4;
    pub const DAMAGE_SUSTAINED: u32 = 8;
}

pub mod offroad_moment_id {
    pub const GAS_STATION: u32 = 1;
    pub const CAMPUS: u32 = 2;
    pub const GOLF_COURSE: u32 = 4;
    pub const HOSPITAL: u32 = 8;
    pub const STRIP_MALL: u32 = 16;
    pub const STADIUM: u32 = 32;
    pub const PARK: u32 = 64;
    pub const TRAILER_PARK: u32 = 128;
    pub const JUNKYARD: u32 = 256;
    pub const BOATYARD: u32 = 512;
    pub const REFINERY: u32 = 1024;
    pub const TRAINYARD: u32 = 2048;
    pub const BOARDWALK: u32 = 4096;
    pub const BEACH: u32 = 8192;
    pub const SUBWAY: u32 = 16384;
    pub const HOTEL: u32 = 32768;
    pub const MUSEUM: u32 = 65536;
    pub const POLICE_STATION: u32 = 131072;
    pub const HYDRO_PLANT: u32 = 262144;
    pub const CONSTRUCTION_YARD: u32 = 524288;
    pub const BUS_STATION: u32 = 1048576;
    pub const DRIVE_IN_THEATRE: u32 = 2097152;
    pub const PENITENTIARY: u32 = 4194304;
    pub const FISHERY: u32 = 8388608;
}

pub mod first_subsequent {
    pub const FIRST_TIME: u32 = 1;
    pub const SUBSEQUENT_TIME: u32 = 2;
}

pub mod address_group_type {
    pub const COLLEGE_TOWN: u32 = 1;
    pub const CITY: u32 = 2;
    pub const COASTAL: u32 = 4;
    pub const ALPINE: u32 = 8;
    pub const GENERIC_ANY: u32 = 16;
}

pub mod heli_self_strategy_type {
    pub const OFFSET_FOLLOE: u32 = 1;
    pub const HELI_ROADBLOCK: u32 = 2;
    pub const RAM: u32 = 4;
    pub const SMOKESCREEN: u32 = 8;
    pub const ROLLING_SPIKES: u32 = 16;
}

pub mod heli_hazard_alert_type {
    pub const WINDY_ROADS: u32 = 1;
    pub const APPROACHING_HIGHWAY: u32 = 2;
    pub const APPROACHING_TUNNEL: u32 = 4;
    pub const APPROACHING_CITY: u32 = 8;
    pub const APPROACHING_AIRPORT: u32 = 16;
    pub const APPROACHING_BLIMP: u32 = 32;
}

pub mod heat_level {
    pub const _2: u32 = 1;
    pub const _3: u32 = 2;
    pub const _4: u32 = 4;
    pub const _5: u32 = 8;
    pub const _6: u32 = 16;
    pub const _7: u32 = 32;
    pub const _8: u32 = 64;
    pub const _9: u32 = 128;
    pub const _10: u32 = 256;
}

pub mod jurisdiction {
    pub const STATE: u32 = 1;
    pub const FEDERAL: u32 = 2;
}

pub mod disp_backup_eta {
    pub const _15SEC: u32 = 1;
    pub const _30SEC: u32 = 2;
    pub const _1MIN: u32 = 4;
    pub const _1MIN30SEC: u32 = 8;
    pub const _2MIN: u32 = 16;
    pub const _2MIN_PLUS: u32 = 32;
}

pub mod spikebelt_position {
    pub const LEFT: u32 = 1;
    pub const CENTER: u32 = 2;
    pub const RIGHT: u32 = 4;
}

pub mod car_type {
    pub const MCLAREN: u32 = 1;
    pub const PORSCHE: u32 = 2;
    pub const FORDGT: u32 = 4;
    pub const VIPER: u32 = 8;
    pub const ASTONMARTIN: u32 = 16;
    pub const CORVETTE: u32 = 32;
    pub const BMW: u32 = 64;
    pub const MERCEDES: u32 = 128;
    pub const AUDI: u32 = 256;
    pub const MITSUBISHI: u32 = 512;
    pub const MUSTANG: u32 = 1024;
    pub const LOTUS: u32 = 2048;
    pub const SUBARU: u32 = 4096;
    pub const CAMARO: u32 = 8192;
    pub const GTO: u32 = 16384;
    pub const NISSAN: u32 = 32768;
    pub const MAZDA: u32 = 65536;
    pub const RENAULT: u32 = 131072;
    pub const LEXUS: u32 = 262144;
    pub const MINI: u32 = 524288;
    pub const VOLKSWAGEN: u32 = 1048576;
    pub const SUV: u32 = 2097152;
    pub const PICKUP: u32 = 4194304;
    pub const PAGANI: u32 = 8388608;
    pub const LAMBORGHINI: u32 = 16777216;
    pub const CHRYSLER: u32 = 33554432;
    pub const OPEL: u32 = 67108864;
    pub const SEDAN: u32 = 134217728;
    pub const SPORTSCAR: u32 = 268435456;
    pub const SUPRA: u32 = 536870912;
    pub const CADILLAC: u32 = 1073741824;
}

pub mod direction {
    pub const NORTHBOUND: u32 = 1;
    pub const SOUTHBOUND: u32 = 2;
    pub const EASTBOUND: u32 = 4;
    pub const WESTBOUND: u32 = 8;
}

pub mod car_color {
    pub const RED: u32 = 1;
    pub const BLACK: u32 = 2;
    pub const WHITE: u32 = 4;
    pub const BLUE: u32 = 8;
    pub const GREEN: u32 = 16;
    pub const ORANGE: u32 = 32;
    pub const SILVER: u32 = 64;
    pub const GOLD: u32 = 128;
    pub const PURPLE: u32 = 256;
    pub const BROWN: u32 = 512;
    pub const YELLOW: u32 = 1024;
    pub const PINK: u32 = 2048;
    pub const BEIGE: u32 = 4096;
}

pub mod speed {
    pub const OVER_SPEED_LIMIT: u32 = 1;
    pub const SPEEDS_IN_EXCESS_OF_100: u32 = 2;
    pub const SPEEDS_IN_EXCESS_OF_120: u32 = 4;
    pub const SPEEDS_IN_EXCESS_OF_140: u32 = 8;
    pub const SPEEDS_IN_EXCESS_OF_160: u32 = 16;
    pub const SPEEDS_IN_EXCESS_OF_180: u32 = 32;
    pub const SPEEDS_IN_EXCESS_OF_200: u32 = 64;
    pub const SPEEDS_IN_EXCESS_OF_220: u32 = 128;
    pub const SPEEDS_IN_EXCESS_OF_240: u32 = 256;
    pub const SPEEDS_IN_EXCESS_OF_260: u32 = 512;
    pub const SPEEDS_IN_EXCESS_OF_280: u32 = 1024;
    pub const SPEEDS_IN_EXCESS_OF_300: u32 = 2048;
}

pub mod location_region {
    pub const COLLEGE_TOWN: u32 = 1;
    pub const COASTAL: u32 = 2;
    pub const COASTAL_EXTRA: u32 = 4;
    pub const CITY: u32 = 8;
}

pub mod measurement {
    pub const GENERIC: u32 = 1;
    pub const IMPERIAL_ONLY: u32 = 2;
    pub const METRIC_ONLY: u32 = 4;
}

pub mod projectile_type {
    pub const SMOKESCREEN: u32 = 1;
    pub const ROLLING_SPIKES: u32 = 2;
}

pub mod on_off_scene {
    pub const ON_SCENE: u32 = 1;
    pub const OFF_SCENE: u32 = 2;
}
