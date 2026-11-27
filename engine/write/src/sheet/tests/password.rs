//! `PasswordHash::sha512`: values computed independently with Python's
//! hashlib, the limits of the password and the spin count, and the order
//! the rules are checked in.

use super::base64::decode;
use crate::sheet::base64::encode;
use crate::sheet::password::check_spin_count;
use crate::sheet::{PasswordHash, SheetError};

const SALT_0_TO_15: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

#[test]
fn the_hash_is_the_one_python_computes() {
    for (password, salt, spin_count, hash) in [
        (
            "password",
            SALT_0_TO_15,
            1,
            "NZDQ+jZ6mR+5ccpyiafYnruBqBmEmQtYT0CwkzKLh4BM7grXD6j4MxoSxmy9vtOUDwD13ydo2XOsckpiBoEAJg==",
        ),
        (
            "password",
            SALT_0_TO_15,
            100_000,
            "x01qKaF9y9cQwPxHrE46zKhOLAHXLgmWjpZRPwqjkl6tpT1Lq9JXlHzPvHxsy/q0gWkWsUumW+mgF2sVqd4VXQ==",
        ),
        (
            "パスワード🔒",
            [0xff; 16],
            1000,
            "2ZHFqeVXhlKRU6BYSGtPXA00UxZBUgAXDo9mph/JJBbialLoPc8D+6Ice1t3/mwK5P/b0+eQLpGjrjA37BHUyQ==",
        ),
        (
            "a",
            [0; 16],
            1,
            "k6NH1h4jSwJKJoUfSNkvCfEJgUg0Pk0WT7Xc4iMXx/i+nvKJoc64atwH0F+b/HZfchS2xIzU3topOpyyAUxv2w==",
        ),
    ] {
        let got = PasswordHash::sha512(password, salt, spin_count).unwrap();
        assert_eq!(encode(&got.hash()), hash, "{password}");
        assert_eq!(got.salt(), salt);
        assert_eq!(got.spin_count(), spin_count);
        assert_eq!(decode(hash), got.hash());
    }
}

#[test]
fn a_password_is_1_to_255_utf16_code_units() {
    let hash = |password: &str| PasswordHash::sha512(password, [0; 16], 1);
    assert_eq!(hash(""), Err(SheetError::PasswordEmpty));
    assert!(hash("a").is_ok());
    assert!(hash(&"a".repeat(255)).is_ok());
    assert_eq!(
        hash(&"a".repeat(256)),
        Err(SheetError::PasswordTooLong { length: 256 })
    );
    // A character past U+FFFF is two code units.
    assert!(hash(&format!("{}a", "🔒".repeat(127))).is_ok());
    assert_eq!(
        hash(&"🔒".repeat(128)),
        Err(SheetError::PasswordTooLong { length: 256 })
    );
    assert_eq!(
        hash(&"🔒".repeat(1000)),
        Err(SheetError::PasswordTooLong { length: 2000 })
    );
}

#[test]
fn a_spin_count_is_1_to_10_million() {
    assert_eq!(
        check_spin_count(0),
        Err(SheetError::SpinCount { spin_count: 0 })
    );
    assert_eq!(check_spin_count(1), Ok(()));
    assert_eq!(check_spin_count(10_000_000), Ok(()));
    let over = SheetError::SpinCount {
        spin_count: 10_000_001,
    };
    assert_eq!(check_spin_count(10_000_001), Err(over));
    let got = PasswordHash::sha512("a", [0; 16], u32::MAX);
    assert_eq!(
        got,
        Err(SheetError::SpinCount {
            spin_count: u32::MAX
        })
    );
    assert_eq!(
        PasswordHash::sha512("a", [0; 16], 0),
        Err(SheetError::SpinCount { spin_count: 0 })
    );
}

#[test]
fn the_password_is_checked_before_the_spin_count() {
    let got = PasswordHash::sha512("", [0; 16], 0);
    assert_eq!(got, Err(SheetError::PasswordEmpty));
    let got = PasswordHash::sha512(&"a".repeat(256), [0; 16], 0);
    assert_eq!(got, Err(SheetError::PasswordTooLong { length: 256 }));
}
