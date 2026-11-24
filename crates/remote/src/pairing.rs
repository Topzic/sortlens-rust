//! The pairing gate: a short-lived code on the screen becomes a device token.
//!
//! The server prints a six-digit code when it starts (and refreshes it every ten minutes).
//! A device that knows the code posts it once to `/pair` and receives a scoped bearer token.
//! Wrong attempts are counted: five and the current code is dead, so a guesser on the LAN
//! cannot grind through it.

use sortlens_core::Error as CoreError;
use sortlens_core::Result;

/// How long a code stays valid, in seconds. The design's ten minutes.
pub const CODE_TTL_SECS: u64 = 600;

/// Wrong attempts a single code tolerates before it locks out.
pub const MAX_ATTEMPTS: u32 = 5;

/// Why a code would not pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairError {
    /// The code is past its ten minutes.
    Expired,
    /// The code is wrong; some attempts remain.
    Wrong { left: u32 },
    /// The code is wrong and the attempts are spent; it is dead either way.
    Locked,
}

impl PairError {
    /// The sentence the server puts in the reply, so a client shows a reason, not a code.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            PairError::Expired => "the pairing code has expired; the server issues a fresh one",
            PairError::Wrong { .. } => "wrong pairing code",
            PairError::Locked => "too many wrong attempts; wait for the next code",
        }
    }
}

/// One live code and the attempts against it.
#[derive(Debug, Clone)]
pub struct PairingGate {
    code: String,
    issued_unix: u64,
    ttl_secs: u64,
    wrong: u32,
}

impl PairingGate {
    /// A fresh gate holding `code`, valid from `now_unix` for `ttl_secs`.
    #[must_use]
    pub fn new(code: impl Into<String>, now_unix: u64, ttl_secs: u64) -> Self {
        Self {
            code: code.into(),
            issued_unix: now_unix,
            ttl_secs,
            wrong: 0,
        }
    }

    /// The current code, for the server to print and write to the pairing file.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Whether the code is past its life.
    #[must_use]
    pub fn expired(&self, now_unix: u64) -> bool {
        now_unix.saturating_sub(self.issued_unix) >= self.ttl_secs
    }

    /// Attempts left before lockout.
    #[must_use]
    pub fn attempts_left(&self) -> u32 {
        MAX_ATTEMPTS.saturating_sub(self.wrong)
    }

    /// Swap in a fresh code and forget the attempts against the old one.
    pub fn rotate(&mut self, code: impl Into<String>, now_unix: u64) {
        self.code = code.into();
        self.issued_unix = now_unix;
        self.wrong = 0;
    }

    /// Try to redeem. The clock is a parameter, so tests walk time without sleeping.
    pub fn redeem(&mut self, code: &str, now_unix: u64) -> std::result::Result<(), PairError> {
        if self.expired(now_unix) {
            return Err(PairError::Expired);
        }
        if self.wrong >= MAX_ATTEMPTS {
            return Err(PairError::Locked);
        }
        if self.code != code {
            self.wrong += 1;
            if self.wrong >= MAX_ATTEMPTS {
                return Err(PairError::Locked);
            }
            return Err(PairError::Wrong {
                left: MAX_ATTEMPTS - self.wrong,
            });
        }
        Ok(())
    }
}

/// Six digits from a local entropy mix.
///
/// The code's job is to be hard to guess within its ten minutes on a LAN, not to survive a
/// nation-state: the server mixes clock nanoseconds, the process id, a stack address and a
/// counter through SHA-256 and takes the low six digits. A caller that pins a code
/// (`--code`) skips this and gets exactly what it asked for.
#[must_use]
pub fn generate_code(entropy: &[u8]) -> String {
    let digest = sortlens_ai::sha256::digest(entropy);
    let number = (u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000) as usize;
    format!("{number:06}")
}

/// The SHA-256-backed token: 32 lowercase hex characters.
#[must_use]
pub fn generate_token(entropy: &[u8]) -> String {
    sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(entropy))
}

/// Validate a server-pinned pairing code argument.
pub fn validate_pinned(code: &str) -> Result<()> {
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(CoreError::Invalid("a pairing code is six digits (--code 123456)".into()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_code_redeems_once_and_wrong_attempts_are_counted() {
        let mut gate = PairingGate::new("123456", 1_000, CODE_TTL_SECS);
        assert!(gate.redeem("123456", 1_000).is_ok(), "the right code pairs");
        for left in (1..=4).rev() {
            assert_eq!(gate.redeem("000000", 1_100), Err(PairError::Wrong { left }), "attempt {left} left");
        }
        assert_eq!(gate.redeem("000000", 1_200), Err(PairError::Locked));
        assert_eq!(gate.attempts_left(), 0);
        // Even the right code is dead once locked.
        assert_eq!(gate.redeem("123456", 1_300), Err(PairError::Locked));
    }

    #[test]
    fn a_code_expires_at_its_ttl_and_a_rotation_resets_the_attempts() {
        let mut gate = PairingGate::new("123456", 1_000, CODE_TTL_SECS);
        assert_eq!(gate.redeem("123456", 1_000 + CODE_TTL_SECS - 1), Ok(()));
        let mut gate = PairingGate::new("123456", 1_000, CODE_TTL_SECS);
        assert_eq!(gate.redeem("123456", 1_000 + CODE_TTL_SECS), Err(PairError::Expired));
        gate.rotate("654321", 5_000);
        assert_eq!(
            gate.redeem("000000", 5_100),
            Err(PairError::Wrong { left: 4 }),
            "fresh code, fresh attempts"
        );
        assert!(gate.redeem("654321", 5_200).is_ok());
    }

    #[test]
    fn the_generated_code_is_six_digits_and_two_entropies_differ() {
        let a = generate_code(b"one mix");
        let b = generate_code(b"another mix");
        assert_eq!(a.len(), 6);
        assert!(a.chars().all(|c| c.is_ascii_digit()));
        assert_ne!(a, b, "different entropy mixes must not collide on the first draw");
        let token = generate_token(b"one mix");
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn a_pinned_code_must_be_six_digits() {
        assert!(validate_pinned("424242").is_ok());
        assert!(validate_pinned("42424").is_err());
        assert!(validate_pinned("42424x").is_err());
    }
}
