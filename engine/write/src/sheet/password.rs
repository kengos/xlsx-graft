//! [`PasswordHash`]: the hash of a sheet protection's password, as
//! ECMA-376-1:2016 §18.3.1.85 describes it for the algorithm
//! `SHA-512`.

use super::SheetError;
use sha2::{Digest, Sha512};

/// The salted, spun SHA-512 of a password, with the salt and the spin
/// count: what `sheetProtection` holds as `hashValue`, `saltValue` and
/// `spinCount`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash {
    hash: [u8; 64],
    salt: [u8; 16],
    spin_count: u32,
}

impl PasswordHash {
    /// The most UTF-16 code units a password has.
    const MAX_LENGTH: usize = 255;

    /// The hash of `password`: `H0` is SHA-512 of the salt followed by the
    /// password in UTF-16, little-endian, and each `H(i+1)` is SHA-512 of
    /// `H(i)` followed by `i` as a 32-bit little-endian integer, for `i`
    /// from 0 to one less than the spin count.
    ///
    /// The salt is the caller's, so the same call gives the same hash; a
    /// caller that wants a random salt generates the 16 bytes and passes them.
    ///
    /// # Errors
    ///
    /// [`SheetError::PasswordEmpty`], then [`SheetError::PasswordTooLong`]
    /// for more than 255 UTF-16 code units, then [`SheetError::SpinCount`]
    /// for a spin count of 0 or past 10,000,000.
    pub fn sha512(password: &str, salt: [u8; 16], spin_count: u32) -> Result<Self, SheetError> {
        let units: Vec<u16> = password.encode_utf16().collect();
        if units.is_empty() {
            return Err(SheetError::PasswordEmpty);
        }
        if units.len() > Self::MAX_LENGTH {
            return Err(SheetError::PasswordTooLong {
                length: units.len(),
            });
        }
        check_spin_count(spin_count)?;
        let mut hash = Sha512::new().chain_update(salt);
        for unit in units {
            hash = hash.chain_update(unit.to_le_bytes());
        }
        let mut digest = hash.finalize();
        for i in 0..spin_count {
            digest = Sha512::new()
                .chain_update(digest)
                .chain_update(i.to_le_bytes())
                .finalize();
        }
        Ok(Self {
            hash: digest.into(),
            salt,
            spin_count,
        })
    }

    /// The 64 bytes of the hash.
    #[must_use]
    pub fn hash(&self) -> [u8; 64] {
        self.hash
    }

    /// The 16 bytes of the salt.
    #[must_use]
    pub fn salt(&self) -> [u8; 16] {
        self.salt
    }

    /// The spin count.
    #[must_use]
    pub fn spin_count(&self) -> u32 {
        self.spin_count
    }
}

/// Refuses a spin count of 0 or above 10,000,000, the largest Excel accepts.
pub(super) fn check_spin_count(spin_count: u32) -> Result<(), SheetError> {
    match spin_count {
        1..=10_000_000 => Ok(()),
        _ => Err(SheetError::SpinCount { spin_count }),
    }
}
