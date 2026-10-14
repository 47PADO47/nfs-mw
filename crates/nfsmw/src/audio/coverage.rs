//! Explicit implementation coverage, independent of observed playback or parity.
use super::{Audio, trace::Record};

pub(super) struct Family {
    pub class: &'static str,
    pub route: &'static str,
    pub limits: &'static str,
}

pub(super) const FAMILIES: &[Family] = &[
    Family { class: "CAR", route: "authored", limits: "original timer/RNG and backend DSP equivalence unverified" },
    Family { class: "CAR_Sputter", route: "authored", limits: "original random sequence unverified" },
    Family { class: "CAR_TRANNY", route: "authored", limits: "backend DSP equivalence unverified" },
    Family { class: "FX_SKID", route: "authored", limits: "damage and full terrain overrides incomplete" },
    Family { class: "FX_WIND", route: "authored", limits: "backend spatial/filter equivalence unverified" },
    Family { class: "FX_WIND_Weather", route: "authored", limits: "backend spatial/filter equivalence unverified" },
    Family { class: "FX_ROADNOISE", route: "authored", limits: "backend DSP equivalence unverified" },
    Family { class: "FX_ROADNOISE_TRANS", route: "authored", limits: "backend DSP equivalence unverified" },
    Family { class: "AEMS_StichCollision", route: "authored", limits: "original random sequence unverified" },
    Family { class: "FX_Scrape", route: "authored", limits: "original random sequence and backend DSP unverified" },
    Family {
        class: "FX_UVES",
        route: "authored",
        limits: "Speedbreaker activation and pursuit breaker events; music filter and mode volume treatment incomplete",
    },
    Family {
        class: "FX_NITROUS",
        route: "authored",
        limits: "backend DSP and controlled original comparison required",
    },
    Family { class: "FX_PURGE", route: "authored", limits: "backend DSP and controlled original comparison required" },
    Family { class: "CAR_SWTN", route: "authored", limits: "controlled original comparison required" },
    Family { class: "FX_SHIFTING_01", route: "authored", limits: "controlled original comparison required" },
    Family {
        class: "FX_TRAFFIC",
        route: "authored",
        limits: "truck auxiliary/scenery drive-bys and controlled original comparison required",
    },
    Family {
        class: "ENV_STATIC",
        route: "authored_horn",
        limits: "other environmental/static callers not implemented",
    },
    Family {
        class: "AEMS_StichWoosh",
        route: "authored",
        limits: "original random sequence and controlled original comparison required",
    },
    Family {
        class: "AEMS_StichStatic",
        route: "missing",
        limits: "radio chirp producer and other static callers not integrated; captured caller attribution unverified",
    },
    Family {
        class: "FX_TURBO_01",
        route: "authored",
        limits: "original random sequence, timer and backend DSP unverified",
    },
    Family { class: "CAR_WHINE", route: "authored", limits: "controlled original comparison required" },
    Family {
        class: "FXEDIT",
        route: "authored_dsp",
        limits: "16-sample FIFO; final host quantization/stereo downmix and live driving comparison remain bounded",
    },
    Family {
        class: "SPEEDBREAKER_AUDIO",
        route: "authored_pitch",
        limits: "shared real-time pitch envelope; music filter, mode volume and matched PCM remain open",
    },
    Family { class: "REMOTE_RACER_COP_AUDIO", route: "missing", limits: "remote car sound producers not implemented" },
];

impl Audio {
    pub(super) fn start_trace(&self, path: &std::path::Path) -> Result<(), String> {
        self.trace.start(path)?;
        for family in FAMILIES {
            self.trace.record(Record {
                event: "coverage",
                class: family.class,
                bank: family.route,
                detail: family.limits,
                logical_ms: self.trace.now_ms(),
                ..Default::default()
            });
        }
        Ok(())
    }
    pub(super) fn coverage(&self) -> String {
        let mut report = String::from("Audio coverage (implementation, not parity):\n");
        for family in FAMILIES {
            report.push_str(&format!("{}: {} - {}\n", family.class, family.route, family.limits));
        }
        report.push_str("audio-trace <path> records actual module/voice coverage; audio-trace off flushes it.\n");
        report
    }
}
