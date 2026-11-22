//! Unit tests for `Prefix`: the text of each prefix and the list of all.

use super::Prefix;

#[test]
fn as_str_gives_the_text_written_before_the_name() {
    assert_eq!(Prefix::Bare.as_str(), Some(""));
    assert_eq!(Prefix::Xlfn.as_str(), Some("_xlfn."));
    assert_eq!(Prefix::XlfnXlws.as_str(), Some("_xlfn._xlws."));
    assert_eq!(Prefix::Unknown.as_str(), None);
}

#[test]
fn all_lists_every_prefix_in_declaration_order() {
    assert_eq!(
        Prefix::ALL,
        [
            Prefix::Bare,
            Prefix::Xlfn,
            Prefix::XlfnXlws,
            Prefix::Unknown
        ]
    );
}
