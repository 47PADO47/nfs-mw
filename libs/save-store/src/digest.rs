//! A file that starts with its own MD5 digest: the digest covers every byte after it (16 bytes, then the body).

use md5::{Digest, Md5};

/// The length of the digest at the start of the file.
pub const DIGEST_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    #[error("the file is shorter than its {DIGEST_LEN}-byte digest")]
    TooShort,
    #[error("the digest does not match the body")]
    Mismatch,
}

/// The MD5 of `body`.
pub fn md5(body: &[u8]) -> [u8; DIGEST_LEN] {
    let mut hasher = Md5::new();
    hasher.update(body);
    hasher.finalize().into()
}

/// Checks the digest at the start of `file` and returns the body (everything after the digest).
pub fn check_prefixed(file: &[u8]) -> Result<&[u8], DigestError> {
    let (digest, body) = file.split_at_checked(DIGEST_LEN).ok_or(DigestError::TooShort)?;
    match digest == md5(body) {
        true => Ok(body),
        false => Err(DigestError::Mismatch),
    }
}

/// Puts the digest of `body` in front of it.
pub fn with_prefix(body: &[u8]) -> Vec<u8> {
    let mut file = Vec::with_capacity(DIGEST_LEN + body.len());
    file.extend_from_slice(&md5(body));
    file.extend_from_slice(body);
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_with_its_digest_checks_and_gives_the_body_back() {
        let file = with_prefix(b"body bytes");
        assert_eq!(check_prefixed(&file), Ok(&b"body bytes"[..]));
    }

    #[test]
    fn a_changed_byte_is_a_mismatch() {
        let mut file = with_prefix(b"body bytes");
        *file.last_mut().unwrap() ^= 1;
        assert_eq!(check_prefixed(&file), Err(DigestError::Mismatch));
    }

    #[test]
    fn a_file_shorter_than_the_digest_is_refused() {
        assert_eq!(check_prefixed(&[0; 15]), Err(DigestError::TooShort));
    }

    #[test]
    fn md5_matches_the_standard_test_vector() {
        // RFC 1321: MD5("") = d41d8cd98f00b204e9800998ecf8427e
        let digest = md5(b"");
        assert_eq!(digest[0], 0xd4);
        assert_eq!(digest[15], 0x7e);
    }
}
