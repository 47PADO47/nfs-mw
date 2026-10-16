//! Panic containment for callbacks (ADR 0005 section 3).
//!
//! Since Rust 1.81 a panic that unwinds out of an `extern "C"` function aborts the process. Every callback the
//! SDK exports runs its body through [`run`], so the panic is caught before it reaches the host.

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::{Error, ModResult};

/// Run one callback body. A panic becomes [`Error::Panic`]; an `Err` passes through unchanged.
pub fn run<T>(body: impl FnOnce() -> ModResult<T>) -> ModResult<T> {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(_payload) => Err(Error::Panic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::BB_ERR_PANIC;

    #[test]
    fn a_value_passes_through() {
        assert_eq!(run(|| Ok(7)), Ok(7));
    }

    #[test]
    fn an_error_passes_through() {
        assert_eq!(run::<()>(|| Err(Error::Stale)), Err(Error::Stale));
    }

    #[test]
    fn a_panic_becomes_the_panic_error_and_code() {
        let result = run(|| -> ModResult<()> { panic!("guard test: expected panic") });
        assert_eq!(result, Err(Error::Panic));
        let code = match result {
            Err(error) => error.code(),
            Ok(()) => 0,
        };
        assert_eq!(code, BB_ERR_PANIC);
    }
}
