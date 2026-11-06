//! What reading and verifying each workbook in `fixtures/` gives.

/// The lines `render.rs` makes of one workbook.
pub(crate) struct Expected {
    /// The workbook's file name in `fixtures/`.
    pub(crate) file: &'static str,
    /// [`render::verify`](crate::render::verify)'s lines.
    pub(crate) verify: &'static [&'static str],
    /// [`render::read`](crate::render::read)'s lines.
    pub(crate) read: &'static [&'static str],
}

/// One entry per workbook in `fixtures/`, sorted by file name.
pub(crate) const EXPECTED: &[Expected] = &[
    // A clustered column chart of Sheet1!A1:B4: its chart and drawing
    // parts raise nothing, and its series formulas are judged as formulas.
    Expected {
        file: "chart.xlsx",
        verify: &[],
        read: &[
            "date system Compatibility1900",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: \"Item\" | \"Qty\"",
            "Sheet1!2: \"A\" | 2",
            "Sheet1!3: \"B\" | 3",
            "Sheet1!4: \"C\" | 5",
        ],
    },
    // The 1904 date system (workbookPr@date1904): serial 0 reads as
    // 1904-01-01 and 43844 as 2024-01-15 under a date format.
    Expected {
        file: "date1904.xlsx",
        verify: &[],
        read: &[
            "date system Base1904",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: \"Serial\" | \"Date\"",
            "Sheet1!2: 0 | Date(Date { year: 1904, month: 1, day: 1 }) | Date(Date { year: 2024, month: 1, day: 15 })",
            "Sheet1!3: 43844 | Date(Date { year: 2024, month: 1, day: 15 })",
            "Sheet1!4: 1.5 | DateTime(DateTime { date: Date { year: 1904, month: 1, day: 2 }, time: Time { hour: 12, minute: 0, second: 0, millisecond: 0 } })",
        ],
    },
    // Formulas that spill (SEQUENCE, SORT, FILTER, UNIQUE) and a spill
    // reference (A1#, saved as ANCHORARRAY): the default function table
    // reviewed each safe, and the default targets all have dynamic arrays.
    Expected {
        file: "dynamic-array.xlsx",
        verify: &[],
        read: &[
            "date system Compatibility1900",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: 1 | 3 | 1 | 3 | 1 | 15",
            "Sheet1!2: 2 | 1 | 2 | 4 | 2",
            "Sheet1!3: 3 | 4 | 3",
            "Sheet1!4: 4 | 2 | 4",
            "Sheet1!5: 5",
        ],
    },
    // Formulas that refer to external-source.xlsx: the external workbook
    // reference part is denied, and so is the first formula that names it.
    Expected {
        file: "external-link.xlsx",
        verify: &[
            "part_denied part=/xl/externalLinks/externalLink1.xml",
            "formula_denied part=/xl/worksheets/sheet1.xml place=A1",
        ],
        read: &[
            "date system Compatibility1900",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: 42 | \"ext\" | 43",
        ],
    },
    // The workbook external-link.xlsx refers to: plain values.
    Expected {
        file: "external-source.xlsx",
        verify: &[],
        read: &[
            "date system Compatibility1900",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: 42 | \"ext\"",
        ],
    },
    // Formulas the lexer's tests were checked against: names, a table's
    // structured references, quoted sheet names and LAMBDA. H24 calls
    // WEBSERVICE, which the default function table classifies dangerous
    // and Excel for Mac, one of the default targets, lacks.
    Expected {
        file: "formulas.xlsx",
        verify: &[
            "target_function_unsupported part=/xl/worksheets/sheet1.xml place=H24",
            "function_dangerous part=/xl/worksheets/sheet1.xml place=H24",
        ],
        read: &[
            "date system Compatibility1900",
            "sheet \"Sheet1\" Worksheet",
            "Sheet1!1: 1 | 1 |  | \"a'#\" | \"a\\\"b\" | \"x[y]\" | \"calc\" | 2 |  | 1",
            "Sheet1!2: 2 | 2 |  | 2 | 2 | 2 | Error(\"#REF!\")",
            "Sheet1!3: 3 | 3 |  | 3 | 3 | 3 | Error(\"#REF!\")",
            "Sheet1!5:  |  |  |  |  |  |  | 1",
            "Sheet1!9:  |  |  |  |  |  |  | 2",
            "Sheet1!10:  |  |  |  |  |  |  | 2",
            "Sheet1!11:  |  |  |  |  |  |  | 2",
            "Sheet1!14:  |  |  |  |  |  |  | 2",
            "Sheet1!17:  |  |  |  |  |  |  | 2",
            "Sheet1!18:  |  |  |  |  |  |  | Error(\"#REF!\")",
            "Sheet1!19:  |  |  |  |  |  |  | 5",
            "Sheet1!20:  |  |  |  |  |  |  | 5",
            "Sheet1!21:  |  |  |  |  |  |  | 2",
            "Sheet1!22:  |  |  |  |  |  |  | 6",
            "Sheet1!24:  |  |  |  |  |  |  | Error(\"#NAME?\")",
            "Sheet1!25:  |  |  |  |  |  |  | 1",
            "Sheet1!26:  |  |  |  |  |  |  | Error(\"#VALUE!\")",
            "Sheet1!27:  |  |  |  |  |  |  | 3",
            "Sheet1!28:  |  |  |  |  |  |  | \"SUM(\\\"x\\\")\"",
            "Sheet1!29:  |  |  |  |  |  |  | 10",
            "sheet \"Sheet2\" Worksheet",
            "Sheet2!1: 5",
            "sheet \"it's\" Worksheet",
            "it's!1: 1",
            "sheet \"a b\" Worksheet",
            "a b!1: 1",
        ],
    },
    // A table with a calculated column and a totals row, and a formula
    // outside it that refers to a column of it.
    Expected {
        file: "table.xlsx",
        verify: &[],
        read: &[
            "date system Compatibility1900",
            "sheet \"Data\" Worksheet",
            "Data!1: \"Item\" | \"Qty\" | \"Price\" | \"Total\" |  | 230",
            "Data!2: \"A\" | 2 | 10 | 20",
            "Data!3: \"B\" | 3 | 20 | 60",
            "Data!4: \"C\" | 5 | 30 | 150",
            "Data!5: \"Sum\" |  |  | 230",
        ],
    },
];
