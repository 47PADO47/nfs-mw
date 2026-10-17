//! The ABI version and the handshake rule (ADR 0005 sections 1 and 7).

/// ABI major version. A mod built for another major is refused.
pub const ABI_MAJOR: u16 = 0;
/// ABI minor version. Tables only grow within a major, so a mod built for an older minor loads on a newer host.
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

/// The two sides do not agree on the ABI version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiMismatch {
    /// The host's version.
    pub host: AbiVersion,
    /// The mod's version.
    pub module: AbiVersion,
}

/// The handshake rule (ADR 0005 section 7):
///
/// - the majors must be equal;
/// - while the major is `0`, the minors must be equal (nothing is promised across minors yet);
/// - from `1.x` on, the host accepts a mod built against an equal or older minor. A mod built against a newer minor
///   may use tables the host does not have, so it is refused.
pub const fn check_handshake(host: AbiVersion, module: AbiVersion) -> Result<(), AbiMismatch> {
    if host.major != module.major {
        return Err(AbiMismatch { host, module });
    }
    if host.major == 0 && host.minor != module.minor {
        return Err(AbiMismatch { host, module });
    }
    if module.minor > host.minor {
        return Err(AbiMismatch { host, module });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn version(major: u16, minor: u16) -> AbiVersion {
        AbiVersion { major, minor }
    }

    #[test]
    fn the_same_version_is_accepted() {
        assert_eq!(check_handshake(ABI_VERSION, ABI_VERSION), Ok(()));
    }

    #[test]
    fn while_major_is_zero_an_older_module_minor_is_rejected() {
        let module = version(0, ABI_MINOR - 1);
        assert_eq!(
            check_handshake(version(0, ABI_MINOR), module),
            Err(AbiMismatch { host: version(0, ABI_MINOR), module })
        );
    }

    #[test]
    fn while_major_is_zero_a_newer_module_minor_is_rejected() {
        let module = version(0, ABI_MINOR + 1);
        assert_eq!(check_handshake(ABI_VERSION, module), Err(AbiMismatch { host: ABI_VERSION, module }));
    }

    #[test]
    fn from_major_one_an_equal_module_minor_is_accepted() {
        assert_eq!(check_handshake(version(1, 3), version(1, 3)), Ok(()));
    }

    #[test]
    fn from_major_one_an_older_module_minor_is_accepted() {
        assert_eq!(check_handshake(version(1, 3), version(1, 0)), Ok(()));
    }

    #[test]
    fn from_major_one_a_newer_module_minor_is_rejected() {
        let module = version(1, 4);
        assert_eq!(check_handshake(version(1, 3), module), Err(AbiMismatch { host: version(1, 3), module }));
    }

    #[test]
    fn a_major_mismatch_is_rejected_in_both_directions() {
        let newer = version(ABI_MAJOR + 1, ABI_MINOR);
        assert_eq!(check_handshake(ABI_VERSION, newer), Err(AbiMismatch { host: ABI_VERSION, module: newer }));
        assert!(check_handshake(newer, ABI_VERSION).is_err());
        assert!(check_handshake(version(1, 0), version(0, 0)).is_err());
    }
}
