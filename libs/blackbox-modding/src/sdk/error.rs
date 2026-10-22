//! SDK errors and their mapping to status codes.

use core::fmt;

use crate::abi::{
    BB_ERR_ABI, BB_ERR_ARG, BB_ERR_CORRUPT, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_HOST, BB_ERR_NOT_FOUND,
    BB_ERR_NOT_IN_CALLBACK, BB_ERR_NOT_READY, BB_ERR_PANIC, BB_ERR_PHASE, BB_ERR_REMOVED, BB_ERR_STALE,
    BB_ERR_UNSUPPORTED, BB_ERR_WRONG_THREAD, BB_OK, BbStatus,
};

/// The result type of SDK calls.
pub type ModResult<T> = Result<T, Error>;

/// An error from an SDK call. Each variant is one `BB_ERR_*` status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The host speaks another ABI major version.
    Abi,
    /// A bad argument.
    Arg,
    /// The host does not offer this call or interface.
    Unsupported,
    /// The handle refers to an object that no longer exists.
    Stale,
    /// The call is not allowed in the current phase.
    Phase,
    /// The call is valid but not possible yet.
    NotReady,
    /// Host calls are valid only inside a mod callback.
    NotInCallback,
    /// Host calls must run on the main thread.
    WrongThread,
    /// The host refused the call.
    Denied,
    /// A named object does not exist.
    NotFound,
    /// A named object already exists.
    Exists,
    /// The call was removed from the ABI and does nothing.
    Removed,
    /// A panic was caught at the boundary.
    Panic,
    /// The host reported an error.
    Host,
    /// A stored blob fails its integrity check (`bb.saves`). The blob is left in place.
    Corrupt,
}

impl Error {
    /// The status code that carries this error across the boundary.
    pub const fn code(self) -> BbStatus {
        match self {
            Error::Abi => BB_ERR_ABI,
            Error::Arg => BB_ERR_ARG,
            Error::Unsupported => BB_ERR_UNSUPPORTED,
            Error::Stale => BB_ERR_STALE,
            Error::Phase => BB_ERR_PHASE,
            Error::NotReady => BB_ERR_NOT_READY,
            Error::NotInCallback => BB_ERR_NOT_IN_CALLBACK,
            Error::WrongThread => BB_ERR_WRONG_THREAD,
            Error::Denied => BB_ERR_DENIED,
            Error::NotFound => BB_ERR_NOT_FOUND,
            Error::Exists => BB_ERR_EXISTS,
            Error::Removed => BB_ERR_REMOVED,
            Error::Panic => BB_ERR_PANIC,
            Error::Host => BB_ERR_HOST,
            Error::Corrupt => BB_ERR_CORRUPT,
        }
    }

    /// The error a status stands for. `None` for `BB_OK` and for any non-negative status. An unknown negative
    /// status maps to [`Error::Host`].
    pub const fn from_status(status: BbStatus) -> Option<Error> {
        if status >= BB_OK {
            return None;
        }
        Some(match status {
            BB_ERR_ABI => Error::Abi,
            BB_ERR_ARG => Error::Arg,
            BB_ERR_UNSUPPORTED => Error::Unsupported,
            BB_ERR_STALE => Error::Stale,
            BB_ERR_PHASE => Error::Phase,
            BB_ERR_NOT_READY => Error::NotReady,
            BB_ERR_NOT_IN_CALLBACK => Error::NotInCallback,
            BB_ERR_WRONG_THREAD => Error::WrongThread,
            BB_ERR_DENIED => Error::Denied,
            BB_ERR_NOT_FOUND => Error::NotFound,
            BB_ERR_EXISTS => Error::Exists,
            BB_ERR_REMOVED => Error::Removed,
            BB_ERR_PANIC => Error::Panic,
            BB_ERR_CORRUPT => Error::Corrupt,
            _ => Error::Host,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Error::Abi => "the host speaks another ABI major version",
            Error::Arg => "bad argument",
            Error::Unsupported => "the host does not offer this",
            Error::Stale => "the handle is stale",
            Error::Phase => "not allowed in this phase",
            Error::NotReady => "not ready",
            Error::NotInCallback => "host calls are valid only inside a mod callback",
            Error::WrongThread => "host calls must run on the main thread",
            Error::Denied => "refused by the host",
            Error::NotFound => "not found",
            Error::Exists => "already exists",
            Error::Removed => "removed from the ABI",
            Error::Panic => "the mod panicked",
            Error::Host => "the host reported an error",
            Error::Corrupt => "the stored data is corrupt",
        };
        f.write_str(text)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::BB_EVENT_CANCEL;

    const ALL: [Error; 14] = [
        Error::Abi,
        Error::Arg,
        Error::Unsupported,
        Error::Stale,
        Error::Phase,
        Error::NotReady,
        Error::NotInCallback,
        Error::WrongThread,
        Error::Denied,
        Error::NotFound,
        Error::Exists,
        Error::Removed,
        Error::Panic,
        Error::Host,
    ];

    #[test]
    fn every_error_maps_to_a_negative_code_and_back() {
        for error in ALL {
            let code = error.code();
            assert!(code < BB_OK, "{error:?} must map to a negative code");
            assert_eq!(Error::from_status(code), Some(error));
        }
    }

    #[test]
    fn success_and_non_error_statuses_are_not_errors() {
        assert_eq!(Error::from_status(BB_OK), None);
        assert_eq!(Error::from_status(BB_EVENT_CANCEL), None);
    }

    #[test]
    fn an_unknown_negative_status_is_a_host_error() {
        assert_eq!(Error::from_status(-999), Some(Error::Host));
    }
}
