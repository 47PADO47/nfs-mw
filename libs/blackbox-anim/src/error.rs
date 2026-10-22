use thiserror::Error;

/// Why an animation payload could not be read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// The payload ends before a field that the format requires.
    #[error("truncated {what} at offset {offset:#x}")]
    Truncated {
        /// The structure that was being read, for example `"skeleton header"`.
        what: &'static str,
        /// The byte offset at which the read ran out of data.
        offset: usize,
    },
    /// The payload does not start with the EAGL4 magic number.
    #[error("not an EAGL4 animation payload: bad magic")]
    BadMagic,
    /// The payload is well formed but uses a feature this reader does not implement.
    #[error("unsupported animation data: {0}")]
    Unsupported(String),
    /// The payload contradicts itself, for example an index past the end of its table.
    #[error("invalid animation data: {0}")]
    Invalid(String),
}

/// The result of reading an animation payload.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn messages_name_the_problem() {
        let truncated = Error::Truncated { what: "skeleton header", offset: 0x18 };
        assert_eq!(truncated.to_string(), "truncated skeleton header at offset 0x18");
        assert_eq!(Error::BadMagic.to_string(), "not an EAGL4 animation payload: bad magic");
        assert_eq!(Error::Unsupported("codec 7".to_string()).to_string(), "unsupported animation data: codec 7");
        assert_eq!(
            Error::Invalid("bone index 40 of 12".to_string()).to_string(),
            "invalid animation data: bone index 40 of 12"
        );
    }
}
