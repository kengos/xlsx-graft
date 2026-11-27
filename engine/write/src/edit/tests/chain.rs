//! Several `PackageEdit::replace` calls for one part: each rewriter reads
//! the output of the one before it, only the first reads the original, the
//! output is the same whether the output between two rewriters stays in
//! memory or goes to a spool, and a refusal or a failure at any rewriter
//! ends the write as it does for one.

use super::replace::{Counting, Noise};
use super::{
    Copy, Refuse, Text, content, contents, edit, entries, finish, found, open, part, written_zip,
};
use crate::tests::{Unreadables, bounded};
use crate::{PackageEdit, PartRewriter, WriteError, WriteLimits};
use std::cell::{Cell, RefCell};
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use xlsx_graft_diagnostics::Diagnostics;
use xlsx_graft_spool::{Spool, SpoolFactory};
use xlsx_graft_xml::events::Event;
use xlsx_graft_xml::{XmlLimits, XmlRewriter};

/// A rewriter that writes the content it reads and then `tail`.
struct Append(&'static str);

impl PartRewriter for Append {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        io::copy(&mut bounded(original), out)?;
        out.write_all(self.0.as_bytes())?;
        Ok(())
    }
}

/// A rewriter that adds the attribute `x="1"` to each empty element and
/// writes every other event as it is read.
struct AddX;

impl PartRewriter for AddX {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut input = bounded(original);
        let mut rewriter = XmlRewriter::new(&mut input, out, &XmlLimits::default());
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

/// A rewriter that reads nothing and counts its calls.
struct Ignore(Rc<Cell<u32>>);

impl PartRewriter for Ignore {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        self.0.set(self.0.get() + 1);
        out.write_all(b"<ignored/>")?;
        Ok(())
    }
}

/// A rewriter that reads the first byte of what it is given and stops.
struct First;

impl PartRewriter for First {
    fn rewrite(
        &mut self,
        original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        let mut byte = [0; 1];
        original.read_exact(&mut byte)?;
        out.write_all(&byte)?;
        Ok(())
    }
}

const WORKBOOK: &str = "/xl/workbook.xml";

#[test]
fn each_rewriter_reads_the_output_of_the_one_before_in_the_order_of_the_calls() {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Text("<w><a/></w>")))
        .unwrap();
    // The second call names the part with other letter case.
    edit.replace(&part("/XL/workbook.xml"), Box::new(AddX))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Append("<!--c-->")))
        .unwrap();
    let bytes = written_zip(edit);
    let written = r#"<w><a x="1"/></w><!--c-->"#;
    assert_eq!(content(&bytes, "xl/workbook.xml"), written);
    // The other entries are as they were, in their order.
    let mut expected = entries();
    expected[3].1 = written.to_owned();
    assert_eq!(contents(&bytes), expected);
    open(bytes);
}

#[test]
fn only_the_first_rewriter_reads_the_original_and_each_is_called_once() {
    let calls = Rc::new(Cell::new(0));
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Text("<first/>")))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Copy(calls.clone())))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Copy(calls.clone())))
        .unwrap();
    let bytes = written_zip(edit);
    assert_eq!(content(&bytes, "xl/workbook.xml"), "<first/>");
    assert_eq!(calls.get(), 2);
}

#[test]
fn an_attribute_a_rewriter_wrote_passes_through_the_next_rewriter_unchanged() {
    let tab = r#"<workbook><sheetView workbookViewId="0" tabSelected="1"/></workbook>"#;
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Text(tab))).unwrap();
    edit.replace(&part(WORKBOOK), Box::new(AddX)).unwrap();
    let bytes = written_zip(edit);
    let expected = r#"<workbook><sheetView workbookViewId="0" tabSelected="1" x="1"/></workbook>"#;
    assert_eq!(content(&bytes, "xl/workbook.xml"), expected);
}

/// The zip written with the part replaced by `Noise(len)` and `then`
/// (called in the order given), and the number of spools the factory made.
fn written_with_spill(
    len: usize,
    spill_at: u64,
    then: Vec<Box<dyn PartRewriter>>,
) -> (Vec<u8>, u32) {
    let made = Rc::new(Cell::new(0));
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Noise(len))).unwrap();
    for rewriter in then {
        edit.replace(&part(WORKBOOK), rewriter).unwrap();
    }
    let limits = WriteLimits {
        part_spill_bytes: spill_at,
        ..WriteLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let written = edit.finish(
        crate::tests::Sink::default(),
        &mut Counting(made.clone()),
        &limits,
        &mut diagnostics,
    );
    (written.unwrap().unwrap().bytes, made.get())
}

#[test]
fn the_output_between_two_rewriters_spills_to_a_spool_past_the_spill_size_and_the_zip_is_the_same()
{
    let copy = || -> Vec<Box<dyn PartRewriter>> { vec![Box::new(Copy(Rc::new(Cell::new(0))))] };
    // Memory only: no spool for the output between the two, nor for the last.
    let (in_memory, made) = written_with_spill(200_000, u64::MAX, copy());
    assert_eq!(made, 0);
    // The output between the two and the last output each take a spool.
    let (spilled, made) = written_with_spill(200_000, 1_000, copy());
    assert_eq!(made, 2);
    assert_eq!(spilled, in_memory);
    // Only the first output is past the spill size, which is what takes one
    // spool: the output of the second rewriter is `<x/>`.
    let then: Vec<Box<dyn PartRewriter>> = vec![Box::new(Text("<x/>"))];
    let (_, made) = written_with_spill(200_000, 1_000, then);
    assert_eq!(made, 1);
    // The zip is the one a single rewriter writes.
    let (single, _) = written_with_spill(200_000, 1_000, Vec::new());
    assert_eq!(spilled, single);
}

/// Spools that are made until the `n`th call of `create`, which fails and
/// every later one.
struct FailingFrom(u32, u32);

impl SpoolFactory for FailingFrom {
    fn create(&mut self) -> io::Result<Box<dyn Spool>> {
        self.1 += 1;
        if self.1 >= self.0 {
            return Err(io::Error::other("no space for a spool"));
        }
        Ok(Box::new(xlsx_graft_spool::MemorySpool::new()))
    }
}

/// The result of finishing the part replaced by `Noise` and then `second`,
/// with spools made until the `fail`th `create`.
fn finish_failing(
    fail: u32,
    second: Box<dyn PartRewriter>,
) -> Result<Option<crate::tests::Sink>, WriteError> {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Noise(200_000)))
        .unwrap();
    edit.replace(&part(WORKBOOK), second).unwrap();
    let limits = WriteLimits {
        part_spill_bytes: 100,
        ..WriteLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    edit.finish(
        crate::tests::Sink::default(),
        &mut FailingFrom(fail, 0),
        &limits,
        &mut diagnostics,
    )
}

#[test]
fn a_spool_that_cannot_be_made_for_either_output_of_a_chain_is_a_write_error() {
    // The first `create` is the spool of the output between the two
    // rewriters, so the second rewriter is not called.
    let calls = Rc::new(Cell::new(0));
    let result = finish_failing(1, Box::new(Copy(calls.clone())));
    assert!(matches!(result, Err(WriteError::Spool(_))), "{result:?}");
    assert_eq!(calls.get(), 0);
    // The second `create` is the spool of the last output: the second
    // rewriter was called, and it is the one whose output cannot be held.
    let result = finish_failing(2, Box::new(Noise(200_000)));
    assert!(matches!(result, Err(WriteError::Spool(_))), "{result:?}");
    // With spools that are never refused, the same chain is written.
    let result = finish_failing(u32::MAX, Box::new(Noise(10)));
    assert!(matches!(result, Ok(Some(_))), "{result:?}");
}

#[test]
fn a_spool_of_the_output_between_two_rewriters_that_cannot_be_read_is_a_write_error() {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Noise(200_000)))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Append("<b/>")))
        .unwrap();
    let limits = WriteLimits {
        part_spill_bytes: 100,
        ..WriteLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let result = edit.finish(
        crate::tests::Sink::default(),
        &mut Unreadables,
        &limits,
        &mut diagnostics,
    );
    assert!(matches!(result, Err(WriteError::Spool(_))), "{result:?}");
}

#[test]
fn a_first_rewriter_that_refuses_the_content_gives_its_diagnostic_and_no_later_rewriter_is_called()
{
    let calls = Rc::new(Cell::new(0));
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Refuse)).unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Copy(calls.clone())))
        .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["xml_doctype /xl/workbook.xml"]);
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_later_rewriter_that_refuses_the_content_gives_its_diagnostic_at_the_part_and_no_zip() {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Text("<first/>")))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Refuse)).unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["xml_doctype /xl/workbook.xml"]);
    assert_eq!(diagnostics.records()[0].diagnostic, Refuse::diagnostic());
}

#[test]
fn a_damaged_original_a_first_rewriter_ignores_is_reported_and_no_later_rewriter_is_called() {
    let mut bytes = super::zip(&entries());
    let data = bytes.windows(11).position(|w| w == b"<workbook/>").unwrap();
    bytes[data + 1] ^= 0xff;
    let calls = Rc::new(Cell::new(0));
    let mut edit = PackageEdit::new(open(bytes));
    edit.replace(&part(WORKBOOK), Box::new(Ignore(calls.clone())))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(Copy(calls.clone())))
        .unwrap();
    let (bytes, diagnostics) = finish(edit);
    assert!(bytes.is_none());
    assert_eq!(found(&diagnostics), ["zip_crc_mismatch xl/workbook.xml"]);
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_later_rewriter_that_reads_part_of_its_input_has_its_output_written_without_an_error() {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Text("<first/>")))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(First)).unwrap();
    let bytes = written_zip(edit);
    assert_eq!(content(&bytes, "xl/workbook.xml"), "<");
}

#[test]
fn a_part_replaced_twice_and_then_removed_is_left_out_and_no_rewriter_is_called() {
    let calls = Rc::new(Cell::new(0));
    let mut edit = edit();
    let sheet = part("/xl/worksheets/sheet1.xml");
    edit.replace(&sheet, Box::new(Copy(calls.clone()))).unwrap();
    edit.replace(&sheet, Box::new(Copy(calls.clone()))).unwrap();
    edit.remove(&sheet).unwrap();
    let bytes = written_zip(edit);
    let names: Vec<String> = contents(&bytes).into_iter().map(|(name, _)| name).collect();
    assert!(!names.contains(&"xl/worksheets/sheet1.xml".to_owned()));
    assert_eq!(calls.get(), 0);
}

#[test]
fn an_xml_rewriter_whose_input_spool_cannot_be_read_fails_the_write_without_a_diagnostic() {
    let mut edit = edit();
    edit.replace(&part(WORKBOOK), Box::new(Noise(200_000)))
        .unwrap();
    edit.replace(&part(WORKBOOK), Box::new(AddX)).unwrap();
    let limits = WriteLimits {
        part_spill_bytes: 100,
        ..WriteLimits::default()
    };
    let mut diagnostics = Diagnostics::new();
    let result = edit.finish(
        crate::tests::Sink::default(),
        &mut Unreadables,
        &limits,
        &mut diagnostics,
    );
    // The XML error keeps the kind and the message of the failure, not its
    // `WriteError::Spool`: it is a failure of the host (`Err`), not a
    // diagnostic about the part.
    assert!(matches!(result, Err(WriteError::Io(_))), "{result:?}");
    assert!(diagnostics.records().is_empty());
}

/// A rewriter that writes `name` to `log` when it runs and when it is
/// dropped.
struct Logged {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

impl PartRewriter for Logged {
    fn rewrite(
        &mut self,
        _original: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> Result<(), WriteError> {
        self.log.borrow_mut().push(format!("run {}", self.name));
        out.write_all(b"<x/>")?;
        Ok(())
    }
}

impl Drop for Logged {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("drop {}", self.name));
    }
}

#[test]
fn the_rewriters_of_a_part_are_dropped_before_the_rewriters_of_a_later_part_are_called() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let logged = |name| {
        Box::new(Logged {
            name,
            log: log.clone(),
        })
    };
    let mut edit = edit();
    // The workbook part is an entry before the worksheet part.
    let sheet = part("/xl/worksheets/sheet1.xml");
    edit.replace(&sheet, logged("sheet 1")).unwrap();
    edit.replace(&sheet, logged("sheet 2")).unwrap();
    edit.replace(&part(WORKBOOK), logged("workbook 1")).unwrap();
    edit.replace(&part(WORKBOOK), logged("workbook 2")).unwrap();
    written_zip(edit);
    let expected = [
        "run workbook 1",
        "run workbook 2",
        "drop workbook 1",
        "drop workbook 2",
        "run sheet 1",
        "run sheet 2",
        "drop sheet 1",
        "drop sheet 2",
    ];
    assert_eq!(*log.borrow(), expected);
}
