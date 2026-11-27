//! [`WorksheetChild`]: the children of a worksheet's root in the order of
//! its schema.

/// Makes the enum, `ALL` and `local_name` from one table, so the order
/// is written once.
macro_rules! worksheet_children {
    ($($variant:ident => $name:literal,)+) => {
        /// A child of a worksheet's root, in the order of ECMA-376-1:2016
        /// §18.3.1.99 (CT_Worksheet): `Ord` is that order.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum WorksheetChild {
            $(
                #[doc = concat!("The `", $name, "` element.")]
                $variant,
            )+
        }

        impl WorksheetChild {
            /// Every child, in the order of the schema.
            pub const ALL: [Self; 39] = [$(Self::$variant,)+];

            /// The element's local name, `sheetPr` for [`Self::SheetPr`].
            #[must_use]
            pub const fn local_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }
    };
}

worksheet_children! {
    SheetPr => "sheetPr",
    Dimension => "dimension",
    SheetViews => "sheetViews",
    SheetFormatPr => "sheetFormatPr",
    Cols => "cols",
    SheetData => "sheetData",
    SheetCalcPr => "sheetCalcPr",
    SheetProtection => "sheetProtection",
    ProtectedRanges => "protectedRanges",
    Scenarios => "scenarios",
    AutoFilter => "autoFilter",
    SortState => "sortState",
    DataConsolidate => "dataConsolidate",
    CustomSheetViews => "customSheetViews",
    MergeCells => "mergeCells",
    PhoneticPr => "phoneticPr",
    ConditionalFormatting => "conditionalFormatting",
    DataValidations => "dataValidations",
    Hyperlinks => "hyperlinks",
    PrintOptions => "printOptions",
    PageMargins => "pageMargins",
    PageSetup => "pageSetup",
    HeaderFooter => "headerFooter",
    RowBreaks => "rowBreaks",
    ColBreaks => "colBreaks",
    CustomProperties => "customProperties",
    CellWatches => "cellWatches",
    IgnoredErrors => "ignoredErrors",
    SmartTags => "smartTags",
    Drawing => "drawing",
    LegacyDrawing => "legacyDrawing",
    LegacyDrawingHF => "legacyDrawingHF",
    DrawingHF => "drawingHF",
    Picture => "picture",
    OleObjects => "oleObjects",
    Controls => "controls",
    WebPublishItems => "webPublishItems",
    TableParts => "tableParts",
    ExtLst => "extLst",
}

impl WorksheetChild {
    /// The child whose local name is `local`, `None` for a name the table
    /// lacks (the case counts). The caller checks the namespace first: only
    /// an element of the main namespace, Transitional or Strict, is looked up.
    #[must_use]
    pub fn from_local_name(local: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|child| child.local_name().as_bytes() == local)
    }

    /// Whether the element may repeat (`maxOccurs="unbounded"`): `cols` and
    /// `conditionalFormatting`.
    #[must_use]
    pub const fn repeats(self) -> bool {
        matches!(self, Self::Cols | Self::ConditionalFormatting)
    }
}
