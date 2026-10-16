//! The ABI version and the handshake rule (ADR 0005 sections 1 and 7).

/// ABI major version. A mod built for another major is refused.
pub const ABI_MAJOR: u16 = 0;
/// ABI minor version. Tables only grow within a major, so either side may be newer.
pub const ABI_MINOR: u16 = 1;

/// A `major.minor` ABI version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiVersion {
    /// Breaking changes. The handshake requires equal majors.
    pub major: u16,
    /// Additive changes. Every table is size-prefixed, so a reader skips what it does not know.
    pub minor: u16,
}

/// The version this crate speaks.
pub const ABI_VERSION: AbiVersion = AbiVersion { major: ABI_MAJOR, minor: ABI_MINOR };

/// The two sides do not speak the same major version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiMismatch {
    /// The host's version.
    pub host: AbiVersion,
    /// The mod's version.
    pub module: AbiVersion,
}

/// The handshake: the majors must be equal. Minors may differ in either direction, because each table carries
/// its size and a reader treats a field it does not find as "not supported".
pub const fn check_handshake(host: AbiVersion, module: AbiVersion) -> Result<(), AbiMismatch> {
    if host.major != module.major {
        return Err(AbiMismatch { host, module });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_version_is_accepted() {
        assert_eq!(check_handshake(ABI_VERSION, ABI_VERSION), Ok(()));
    }

    #[test]
    fn an_older_module_minor_is_accepted() {
        let module = AbiVersion { major: ABI_MAJOR, minor: 0 };
        assert_eq!(check_handshake(ABI_VERSION, module), Ok(()));
    }

    #[test]
    fn a_newer_module_minor_is_accepted() {
        let module = AbiVersion { major: ABI_MAJOR, minor: ABI_MINOR + 1 };
        assert_eq!(check_handshake(ABI_VERSION, module), Ok(()));
    }

    #[test]
    fn a_major_mismatch_is_rejected_in_both_directions() {
        let newer = AbiVersion { major: ABI_MAJOR + 1, minor: ABI_MINOR };
        let older = AbiVersion { major: ABI_MAJOR.wrapping_sub(1), minor: ABI_MINOR };
        assert_eq!(check_handshake(ABI_VERSION, newer), Err(AbiMismatch { host: ABI_VERSION, module: newer }));
        assert!(check_handshake(newer, ABI_VERSION).is_err());
        assert!(check_handshake(ABI_VERSION, older).is_err());
    }
}
