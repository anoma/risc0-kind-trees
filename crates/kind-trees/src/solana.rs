//! A Solana account address: a 32-byte key written in base58, as program ids and mints are written everywhere
//! on Solana. Kept as bytes here so the library carries no Solana SDK; code that holds an SDK `Pubkey`
//! converts through the bytes.
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::fmt;
use std::str::FromStr;

/// A 32-byte Solana address.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
pub struct SolanaAddress([u8; 32]);

impl SolanaAddress {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl FromStr for SolanaAddress {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(text)
            .into_vec()
            .map_err(|error| format!("{text}: not base58: {error}"))?;
        <[u8; 32]>::try_from(bytes)
            .map(Self)
            .map_err(|bytes| format!("{text}: decodes to {} bytes, not 32", bytes.len()))
    }
}

impl fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&bs58::encode(self.0).into_string())
    }
}

impl fmt::Debug for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SolanaAddress({self})")
    }
}
