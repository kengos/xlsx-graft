//! `PackageEdit::replace`: the part written from its rewriter and every
//! other entry unchanged, a rewriter that reads the original, one that
//! refuses the content, and content that spills to a spool.

use super::{
    Refuse, Text, content, contents, deflated_zip, edit, entries, finish, found, open, part,
    written_zip,
};
use crate::{PackageEdit, PartRewriter, WriteError, WriteLimits};
use std::cell::Cell;
use std::io::{BufRead, Write};
use std::rc::Rc;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_spool::{MemorySpool, Spool, SpoolFactory};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// A rewriter that adds the attribute `x="1"` to each empty element, with
/// an `XmlRewriter` over the original.
struct AddAttribute;

impl PartRewriter for AddAttribute {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut rewriter = XmlRewriter::new(original, out, &XmlLimits::default());
        let mut buf = Vec::new();
        while let Some(event) = rewriter.next(&mut buf)? {
            match event {
                Event::Empty(start) => {
                    let changed = start.to_owned().with_attributes([("x", "1")]);
                    rewriter.write(Event::Empty(changed))?;
                }
                event => rewriter.write(event)?,
            }
        }
        Ok(())
    }
}

#[test]
fn a_part_replaced_is_written_from_its_rewriter_and_every_other_entry_is_unchanged() {
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
        .unwrap();
    let bytes = written_zip(edit);
    let mut expected = entries();
    expected[3].1 = "<new/>".to_owned();
    assert_eq!(contents(&bytes), expected);
    // The package still reads, and the entries are in the original order.
    open(bytes);
}

#[test]
fn entries_deflated_in_the_original_keep_their_compressed_bytes() {
    let original = deflated_zip(&entries());
    let mut edit = PackageEdit::new(open(original.clone()));
    edit.replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
        .unwrap();
    let bytes = written_zip(edit);
    let before = crate::tests::open(original);
    let after = crate::tests::open(bytes);
    for (index, entry) in before.entries().iter().enumerate() {
        let (then, now) = (entry, &after.entries()[index]);
        assert_eq!(then.name(), now.name());
        if index == 3 {
            continue;
        }
        assert_eq!((then.method(), then.crc32()), (now.method(), now.crc32()));
        assert_eq!(then.compressed_size(), now.compressed_size());
    }
}

#[test]
fn a_rewriter_reads_the_original_and_writes_its_change() {
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(AddAttribute))
        .unwrap();
    let bytes = written_zip(edit);
    assert_eq!(content(&bytes, "xl/workbook.xml"), r#"<workbook x="1"/>"#);
}

#[test]
fn a_rewriter_that_refuses_the_content_gives_its_diagnostic_at_the_part_and_no_zip() {
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(Refuse))
        .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["xml_doctype /xl/workbook.xml"]);
    assert_eq!(diagnostics.records()[0].diagnostic, Refuse::diagnostic());
}

#[test]
fn a_rewriter_that_ignores_the_original_still_has_the_size_and_crc32_of_the_entry_checked() {
    let mut bytes = super::zip(&entries());
    // Flip a byte of the stored data of `xl/workbook.xml`.
    let data = bytes.windows(11).position(|w| w == b"<workbook/>").unwrap();
    bytes[data + 1] ^= 0xff;
    let mut edit = PackageEdit::new(open(bytes));
    edit.replace(&part("/xl/workbook.xml"), Box::new(Text("<new/>")))
        .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["zip_crc_mismatch xl/workbook.xml"]);
}

#[test]
fn a_rewriter_that_reads_a_damaged_original_ends_the_write_with_the_diagnostic_of_the_entry() {
    let mut bytes = super::zip(&entries());
    let data = bytes.windows(11).position(|w| w == b"<workbook/>").unwrap();
    bytes[data + 1] ^= 0xff;
    let mut edit = PackageEdit::new(open(bytes));
    edit.replace(&part("/xl/workbook.xml"), Box::new(AddAttribute))
        .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics).len(), 1);
}

/// A spool factory that counts the spools it makes.
pub(super) struct Counting(pub(super) Rc<Cell<u32>>);

impl SpoolFactory for Counting {
    fn create(&mut self) -> std::io::Result<Box<dyn Spool>> {
        self.0.set(self.0.get() + 1);
        Ok(Box::new(MemorySpool::new()))
    }
}

/// A rewriter that writes `len` bytes that deflate poorly.
pub(super) struct Noise(pub(super) usize);

impl PartRewriter for Noise {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        out.write_all(&crate::tests::noise(self.0))?;
        Ok(())
    }
}

/// The zip written with the part replaced by `len` bytes of noise, and how
/// many spools that took, under `spill_at`.
fn spilled(len: usize, spill_at: u64) -> (Vec<u8>, u32) {
    let made = Rc::new(Cell::new(0));
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(Noise(len)))
        .unwrap();
    let mut diagnostics = Diagnostics::new();
    let limits = WriteLimits {
        part_spill_bytes: spill_at,
        ..WriteLimits::default()
    };
    let written = edit.finish(
        crate::tests::Sink::default(),
        &mut Counting(made.clone()),
        &limits,
        &mut diagnostics,
    );
    (written.unwrap().unwrap().bytes, made.get())
}

#[test]
fn a_part_larger_than_the_spill_size_goes_to_a_spool_and_is_written_the_same() {
    let (in_memory, made) = spilled(200_000, u64::MAX);
    assert_eq!(made, 0);
    let (spilled, made) = spilled(200_000, 1_000);
    assert_eq!(made, 1);
    assert_eq!(spilled, in_memory);
    // The spill size is the limit given, not the default's.
    let (_, made) = self::spilled(200_000, WriteLimits::default().part_spill_bytes);
    assert_eq!(made, 0);
}

#[test]
fn a_spool_that_cannot_be_made_is_a_write_error() {
    struct Failing;
    impl SpoolFactory for Failing {
        fn create(&mut self) -> std::io::Result<Box<dyn Spool>> {
            Err(std::io::Error::other("no space for a spool"))
        }
    }
    let mut edit = edit();
    edit.replace(&part("/xl/workbook.xml"), Box::new(Noise(200_000)))
        .unwrap();
    let limits = WriteLimits {
        part_spill_bytes: 100,
        ..WriteLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let result = edit.finish(
        crate::tests::Sink::default(),
        &mut Failing,
        &limits,
        &mut diagnostics,
    );
    assert!(matches!(result, Err(WriteError::Spool(_))), "{result:?}");
}

#[test]
fn finishing_twice_the_same_edit_writes_the_same_bytes() {
    let make = || {
        let mut edit = edit();
        edit.replace(&part("/xl/workbook.xml"), Box::new(AddAttribute))
            .unwrap();
        written_zip(edit)
    };
    assert_eq!(make(), make());
}
