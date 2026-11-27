//! The base64 text of bytes (RFC 4648 §4, the standard alphabet with `=`
//! padding), the text form of `xsd:base64Binary`, the type of `hashValue`
//! and `saltValue`.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The base64 text of `bytes`.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for chunk in bytes.chunks(3) {
        let bits = chunk
            .iter()
            .fold(0_u32, |bits, &byte| bits << 8 | u32::from(byte))
            << (8 * (3 - chunk.len()));
        // A chunk of n bytes is n + 1 characters, then padding.
        for (position, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
            let index = usize::try_from(bits >> shift & 63).unwrap_or_default();
            let letter = ALPHABET.get(index).copied().unwrap_or(b'=');
            text.push(char::from(if position <= chunk.len() {
                letter
            } else {
                b'='
            }));
        }
    }
    text
}
