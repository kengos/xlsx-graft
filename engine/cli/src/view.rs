//! What `tree` and `describe-sheet` print the same way: the host's views, in
//! JSON as the `Serialize` of `View` writes them (the JSON `View::to_json`
//! writes) and in text as the host's text types write them, and the exit
//! status of a view.

use crate::{Outcome, Report};
use std::io::{self, Write};
use xlsx_graft_host::{Grid, GridText, Tree, TreeText, View};

impl Report for View<Tree> {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, "{}", TreeText(self))
    }
}

impl Report for View<Grid> {
    fn write_text(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, "{}", GridText(self))
    }
}

/// The exit status of a view that was printed: success when the view is
/// `ok`, failure when it is not, whether it has an `error` or only
/// diagnostics that are errors.
pub(crate) fn outcome<T>(view: &View<T>) -> Outcome {
    if view.ok {
        Outcome::Success
    } else {
        Outcome::Failure
    }
}
