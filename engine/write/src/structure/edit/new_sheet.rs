//! [`NewSheet`]: the content of a sheet added without one, an empty
//! worksheet in the namespaces of the workbook.

use super::ns::{MAIN, RELATIONSHIPS, flavor};
use crate::{PartWriter, WriteError};
use std::io::Write;

/// An empty worksheet.
pub(super) struct NewSheet {
    /// Whether the workbook is Strict.
    pub(super) strict: bool,
}

impl PartWriter for NewSheet {
    fn write(&mut self, out: &mut dyn Write) -> Result<(), WriteError> {
        let at = flavor(self.strict);
        let main = MAIN.get(at).copied().unwrap_or_default();
        let rels = RELATIONSHIPS.get(at).copied().unwrap_or_default();
        let text = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{main}\" xmlns:r=\"{rels}\"><sheetData/></worksheet>"
        );
        Ok(out.write_all(text.as_bytes())?)
    }
}
