use std::fmt;
use std::str::FromStr;

/// A cryptographically random invitation token — 32 bytes, lowercase hex-encoded (64 chars).
///
/// Held in `UnbillConsole` memory only. Never persisted or synced.
/// See unbill-docs/symmetric-channel.md for the invitation flow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct InviteToken(String);

impl InviteToken {
    // sirno:witness:unbill-model:begin
    /// Generate a new token from 32 OS-random bytes.
    ///
    /// Returns the system RNG error if secure randomness is unavailable.
    pub fn generate() -> Result<Self, rand::rngs::SysError> {
        use rand::TryRng as _;
        let mut bytes = [0u8; 32];
        rand::rngs::SysRng.try_fill_bytes(&mut bytes)?;
        Ok(Self(bytes.iter().map(|b| format!("{b:02x}")).collect()))
    }
    // sirno:witness:unbill-model:end

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InviteToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error returned when parsing an invalid invite token string.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidInviteToken;

impl fmt::Display for InvalidInviteToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid invite token: expected 64 lowercase hex characters"
        )
    }
}

impl FromStr for InviteToken {
    type Err = InvalidInviteToken;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(s.to_owned()))
        } else {
            Err(InvalidInviteToken)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is_64_hex_chars() -> Result<(), rand::rngs::SysError> {
        let tok = InviteToken::generate()?;
        let s = tok.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        Ok(())
    }

    #[test]
    fn test_generate_is_unique() -> Result<(), rand::rngs::SysError> {
        assert_ne!(InviteToken::generate()?, InviteToken::generate()?);
        Ok(())
    }

    #[test]
    fn test_round_trip_from_str() -> Result<(), rand::rngs::SysError> {
        let tok = InviteToken::generate()?;
        assert_eq!(tok.to_string().parse(), Ok(tok));
        Ok(())
    }

    #[test]
    fn test_rejects_wrong_length() {
        assert!("abc".parse::<InviteToken>().is_err());
    }

    #[test]
    fn test_rejects_uppercase_hex() {
        let upper = "A".repeat(64);
        assert!(upper.parse::<InviteToken>().is_err());
    }

    #[test]
    fn test_rejects_non_hex() {
        let bad = "z".repeat(64);
        assert!(bad.parse::<InviteToken>().is_err());
    }
}
