//! The function rows the table checks against a template's targets
//! (`FUNCTIONS`): one per function Excel 2010 or later added that the
//! research of 2026-10-20 lists or [MS-XLSX] gives an `_xlfn.` prefix,
//! named without the `_xlfn.` / `_xlws.` prefix, but for ANCHORARRAY and
//! SINGLE, whose versions the dynamic_array feature's row gives, and
//! COPILOT, which Excel for Microsoft 365 removed.
//!
//! A function ECMA-376-1:2016 §18.17.7 defines that has no row in
//! `FUNCTIONS`, such as `SUM` and `IFERROR`, which Excel 2007 already had,
//! is in the `unchecked` block at the end with its safety only: the table
//! does not check it against a template's targets yet, and
//! [`crate::function`] gives no row for it. ANCHORARRAY, SINGLE, CALL,
//! REGISTER.ID and COPILOT, which ECMA-376 does not define, are in the
//! `unchecked` block too.
//!
//! Every function's `safety:` is `unreviewed()` but for the dangerous
//! ones: those that use the network (WEBSERVICE, IMAGE, STOCKHISTORY,
//! TRANSLATE, DETECTLANGUAGE, PY, COPILOT), read a source outside the
//! workbook (RTD and the CUBE functions), run code outside the workbook
//! (CALL, REGISTER.ID), return information about the machine or the
//! workbook's file (INFO, CELL) or open a link (HYPERLINK).
//!
//! The columns are excel, excel-mac, libreoffice, google-sheets, numbers,
//! onlyoffice. A cell no cited source
//! settles is `unk()`, never a guess: a function missing from Google
//! Sheets' list is `unk()` rather than `no()`, since the list lags the help
//! pages. A function an application had before the first version of its
//! list is written as added in that first version. Every row that writes
//! `xl()` cites `MS_XLFN`, the page that says what an older Excel shows.
//!
//! Where the Excel column differs from the research, the functions by
//! category list was read again on 2026-11-04: it gives 2013 for SHEET
//! and SHEETS, and no version for ARABIC, CEILING.PRECISE, FLOOR.PRECISE
//! and HYPGEOM.DIST (the last missing from the research, an `_xlfn.`
//! function of [MS-XLSX]). AGGREGATE's own page lists Excel 2016 and
//! later.
//!
//! line-budget-exempt: the function table is data, one row per function,
//! and grows by a row with every function; splitting it would split the
//! one place a reader scans for a function.

use super::sources::{
    GS_LIST, GS_XMATCH, LO_25_8, LO_HELP, LO_TDF_138749, MS_AGGREGATE, MS_ENCODEURL, MS_FILTERXML,
    MS_FN, MS_GROUPBY, MS_IMAGE, MS_NEW_2021, MS_NEW_2024, MS_REGEXEXTRACT, MS_SWITCH,
    MS_WEBSERVICE, MS_XLFN, MS_XLSX_FN, NUM_14_4, NUM_FEATURES, NUM_ICLOUD, NUM_XLOOKUP,
    NUM_XMATCH, OO_CL, OO_SRC,
};
use super::{Feature, YES, between, dangerous, le, no, on, s, unk, unreviewed, xl};
use crate::app::{
    Excel as X, ExcelMac as M, GoogleSheets as G, LibreOffice as L, Numbers as N, OnlyOffice as O,
};
use crate::entry::{Entry, Fallback::NameError};
use crate::row::{Row, Support};
use crate::safety::{Danger, SafetyRow};

/// Excel 2010's statistical and date functions.
const E2010: Support = s(
    xl(X::V2010),
    xl(M::V2016),
    on(L::V24_2),
    YES,
    unk(),
    on(O::V8_0),
);
/// Excel 2013's functions.
const E2013: Support = s(
    xl(X::V2013),
    xl(M::V2016),
    on(L::V24_2),
    YES,
    unk(),
    on(O::V8_0),
);
/// In today's Excel, which the functions by category list gives no
/// version for; Excel 2007 lacks them, as their `_xlfn.` prefix says. The
/// same holds for Excel for Mac, known only in 365.
const E_UNDATED: Support = s(
    between(X::V2007, X::V365).fallback(NameError),
    le(M::V365),
    on(L::V24_2),
    YES,
    unk(),
    on(O::V8_0),
);
/// Excel 2016's forecasting functions.
const E2016: Support = s(xl(X::V2016), xl(M::V2016), unk(), unk(), unk(), unk());
/// Excel 2024's array functions, also in LibreOffice 25.8.
const E2024_ARRAY: Support = s(
    xl(X::V2024),
    xl(M::V2024),
    on(L::V25_8),
    YES,
    unk(),
    on(O::V8_0),
);
/// Excel 2024's functions for LAMBDA, which LibreOffice lacks.
const E2024_LAMBDA: Support = s(xl(X::V2024), xl(M::V2024), no(), YES, unk(), unk());
/// Microsoft 365's functions no other application has been seen with.
const E365: Support = s(xl(X::V365), xl(M::V365), no(), unk(), unk(), unk());
/// Microsoft 365's functions no source says anything else about.
const E365_ONLY: Support = s(xl(X::V365), xl(M::V365), unk(), unk(), unk(), unk());

/// Excel for Mac lacks it: it relies on Windows.
const MAC_WINDOWS_ONLY: Entry<M> =
    Entry::never().note("relies on Windows; Excel for Mac does not return its result");
/// LibreOffice has no function of the name.
const LO_OWN_REGEX: Entry<L> =
    Entry::never().note("no function of the name; LibreOffice's own REGEX (6.2) differs");
/// Google Sheets has a REGEX function of the name.
const GS_REGEX: Entry<G> =
    YES.partial("a function of the name whose arguments and results differ from Excel's");

/// Excel for Windows' functions by category gives 2016 for SWITCH.
const SWITCH_2016: &str = "the functions by category list gives 2016, the function's page Office 2019 \
    or Microsoft 365; the perpetual 2016 is taken as lacking it";
/// Numbers' version for XLOOKUP and XMATCH.
const NUMBERS_ICLOUD: &str = "Numbers for iCloud added it in 2020; the Mac version is not known";

const S2010: &[&str] = &[MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC];
const S_UNDATED: &[&str] = &[MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC];
const S2013: &[&str] = &[MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC];
const S2016: &[&str] = &[MS_FN, MS_XLSX_FN, MS_XLFN, GS_LIST];
const S2019: &[&str] = &[
    MS_FN,
    MS_XLSX_FN,
    MS_XLFN,
    LO_HELP,
    GS_LIST,
    NUM_FEATURES,
    OO_CL,
    OO_SRC,
];
const S2021: &[&str] = &[
    MS_FN,
    MS_NEW_2021,
    MS_XLSX_FN,
    MS_XLFN,
    LO_HELP,
    GS_LIST,
    NUM_14_4,
    OO_CL,
    OO_SRC,
];
const S2024: &[&str] = &[
    MS_FN,
    MS_NEW_2024,
    MS_XLSX_FN,
    MS_XLFN,
    LO_25_8,
    GS_LIST,
    OO_CL,
];
const S2024_LAMBDA: &[&str] = &[
    MS_FN,
    MS_NEW_2024,
    MS_XLSX_FN,
    MS_XLFN,
    LO_TDF_138749,
    GS_LIST,
    NUM_14_4,
];
const S365: &[&str] = &[MS_FN, MS_GROUPBY, MS_XLFN, LO_HELP];
const S365_REGEX: &[&str] = &[MS_FN, MS_REGEXEXTRACT, MS_XLFN, LO_HELP, GS_LIST, OO_CL];

#[rustfmt::skip]
functions! {
    // Excel 2010.
    "BETA.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "BETA.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "BINOM.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "BINOM.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CHISQ.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CHISQ.DIST.RT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CHISQ.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CHISQ.INV.RT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CHISQ.TEST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CONFIDENCE.NORM" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "CONFIDENCE.T" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "COVARIANCE.P" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "COVARIANCE.S" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "ERF.PRECISE" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "ERFC.PRECISE" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "EXPON.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "F.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "F.DIST.RT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "F.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "F.INV.RT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "F.TEST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "GAMMA.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "GAMMA.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "GAMMALN.PRECISE" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "LOGNORM.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "LOGNORM.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "MODE.MULT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "MODE.SNGL" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NEGBINOM.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NETWORKDAYS.INTL" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NORM.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NORM.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NORM.S.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "NORM.S.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "PERCENTILE.EXC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "PERCENTILE.INC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "PERCENTRANK.EXC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "PERCENTRANK.INC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "POISSON.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "QUARTILE.EXC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "QUARTILE.INC" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "RANK.AVG" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "RANK.EQ" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "STDEV.P" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "STDEV.S" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.DIST.2T" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.DIST.RT" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.INV" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.INV.2T" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "T.TEST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "VAR.P" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "VAR.S" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "WEIBULL.DIST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "WORKDAY.INTL" => E2010, requires: [], sources: S2010, safety: unreviewed();
    "Z.TEST" => E2010, requires: [], sources: S2010, safety: unreviewed();
    // No version in the functions by category list; AGGREGATE's own page starts at 2016.
    "AGGREGATE" => s(between(X::V2007, X::V2016).fallback(NameError), le(M::V2021), on(L::V24_2), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_AGGREGATE, MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: unreviewed();
    "ARABIC" => E_UNDATED, requires: [], sources: S_UNDATED, safety: unreviewed();
    "CEILING.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED, safety: unreviewed();
    "FLOOR.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED, safety: unreviewed();
    "HYPGEOM.DIST" => E_UNDATED, requires: [], sources: S_UNDATED, safety: unreviewed();
    // Excel 2013.
    "ACOT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "ACOTH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BASE" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BINOM.DIST.RANGE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013, safety: unreviewed();
    "BITAND" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BITLSHIFT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BITOR" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BITRSHIFT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "BITXOR" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "CEILING.MATH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "COMBINA" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "COT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "COTH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "CSC" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "CSCH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "DAYS" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "DECIMAL" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "ENCODEURL" => E2013.with_excel_mac(MAC_WINDOWS_ONLY), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_ENCODEURL, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: unreviewed();
    "FILTERXML" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_FILTERXML, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: unreviewed();
    "FLOOR.MATH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "FORMULATEXT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "GAMMA" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "GAUSS" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IFNA" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMCOSH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMCOT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMCSC" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMCSCH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMSEC" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMSECH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMSINH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "IMTAN" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "ISFORMULA" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "ISO.CEILING" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "ISOWEEKNUM" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "MUNIT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "NUMBERVALUE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013, safety: unreviewed();
    "PDURATION" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "PERMUTATIONA" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "PHI" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "RRI" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "SEC" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "SECH" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "SHEET" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "SHEETS" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "SKEW.P" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "UNICHAR" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "UNICODE" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "WEBSERVICE" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_WEBSERVICE, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: dangerous(Danger::Network);
    "XOR" => E2013, requires: [], sources: S2013, safety: unreviewed();
    // Excel 2016.
    "FORECAST.ETS" => E2016, requires: [], sources: S2016, safety: unreviewed();
    "FORECAST.ETS.CONFINT" => E2016, requires: [], sources: S2016, safety: unreviewed();
    "FORECAST.ETS.SEASONALITY" => E2016, requires: [], sources: S2016, safety: unreviewed();
    "FORECAST.ETS.STAT" => E2016, requires: [], sources: S2016, safety: unreviewed();
    "FORECAST.LINEAR" => E2016.with_google_sheets(YES), requires: [], sources: S2016, safety: unreviewed();
    // Excel 2019.
    "CONCAT" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), le(O::V10_0)), requires: [], sources: S2019, safety: unreviewed();
    "IFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: unreviewed();
    "MAXIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: unreviewed();
    "MINIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: unreviewed();
    "SWITCH" => s(xl(X::V2019).note(SWITCH_2016), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_SWITCH, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_FEATURES, OO_CL], safety: unreviewed();
    "TEXTJOIN" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: S2019, safety: unreviewed();
    // Excel 2021.
    "ARRAYTOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, OO_CL], safety: unreviewed();
    "FILTER" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "LET" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), unk()), requires: [], sources: S2021, safety: unreviewed();
    "RANDARRAY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "SEQUENCE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "SORT" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "SORTBY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), unk(), unk(), le(O::V10_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "UNIQUE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: unreviewed();
    "VALUETOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN], safety: unreviewed();
    "XLOOKUP" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_ICLOUD, NUM_XLOOKUP, OO_CL], safety: unreviewed();
    "XMATCH" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, LO_HELP, GS_XMATCH, NUM_ICLOUD, NUM_XMATCH, OO_CL], safety: unreviewed();
    // Excel 2024.
    "BYCOL" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: unreviewed();
    "BYROW" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: unreviewed();
    "CHOOSECOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "CHOOSEROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "DROP" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "EXPAND" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "HSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "IMAGE" => s(xl(X::V2024), xl(M::V2024), no(), YES.partial("its arguments differ from Excel's"), unk(), unk()), requires: [], sources: &[MS_FN, MS_IMAGE, MS_NEW_2024, MS_XLFN, LO_HELP, GS_LIST], safety: dangerous(Danger::Network);
    "ISOMITTED" => s(xl(X::V2024), xl(M::V2024), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2024, MS_XLFN], safety: unreviewed();
    "LAMBDA" => E2024_LAMBDA.with_libreoffice(no().note("a workbook with LAMBDA is reported to show Err:508")).with_numbers(on(N::V14_4)), requires: [], sources: S2024_LAMBDA, safety: unreviewed();
    "MAKEARRAY" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: unreviewed();
    "MAP" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: unreviewed();
    "REDUCE" => E2024_LAMBDA, requires: [], sources: S2024_LAMBDA, safety: unreviewed();
    "SCAN" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: unreviewed();
    "TAKE" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "TEXTAFTER" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024, safety: unreviewed();
    "TEXTBEFORE" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024, safety: unreviewed();
    "TEXTSPLIT" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "TOCOL" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "TOROW" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "VSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "WRAPCOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    "WRAPROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: unreviewed();
    // Microsoft 365 only.
    "DETECTLANGUAGE" => E365_ONLY.with_google_sheets(YES.note("a function of the name is in the list; its results were not compared with Excel's")), requires: [], sources: &[MS_FN, MS_XLFN, GS_LIST], safety: dangerous(Danger::Network);
    "FIELDVALUE" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN], safety: unreviewed();
    "GROUPBY" => E365, requires: [DynamicArray], sources: S365, safety: unreviewed();
    "PERCENTOF" => E365, requires: [], sources: S365, safety: unreviewed();
    "PIVOTBY" => E365, requires: [DynamicArray], sources: S365, safety: unreviewed();
    "PY" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "REGEXEXTRACT" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: unreviewed();
    "REGEXREPLACE" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: unreviewed();
    "REGEXTEST" => E365.with_libreoffice(LO_OWN_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: unreviewed();
    "STOCKHISTORY" => E365_ONLY, requires: [DynamicArray], sources: &[MS_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "TRANSLATE" => E365_ONLY, requires: [], sources: &[MS_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "TRIMRANGE" => E365, requires: [DynamicArray], sources: S365, safety: unreviewed();
    unchecked {
        // The functions of ECMA-376-1:2016 §18.17.7 in its order, but for
        // ISO.CEILING, NETWORKDAYS.INTL and WORKDAY.INTL, which Excel added
        // later: their rows are in the Excel 2010 and Excel 2013 groups.
        // Which Excel first had each function of this block is not checked
        // yet.
        "ABS" => safety: unreviewed();
        "ACCRINT" => safety: unreviewed();
        "ACCRINTM" => safety: unreviewed();
        "ACOS" => safety: unreviewed();
        "ACOSH" => safety: unreviewed();
        "ADDRESS" => safety: unreviewed();
        "AMORDEGRC" => safety: unreviewed();
        "AMORLINC" => safety: unreviewed();
        "AND" => safety: unreviewed();
        "AREAS" => safety: unreviewed();
        "ASC" => safety: unreviewed();
        "ASIN" => safety: unreviewed();
        "ASINH" => safety: unreviewed();
        "ATAN" => safety: unreviewed();
        "ATAN2" => safety: unreviewed();
        "ATANH" => safety: unreviewed();
        "AVEDEV" => safety: unreviewed();
        "AVERAGE" => safety: unreviewed();
        "AVERAGEA" => safety: unreviewed();
        "AVERAGEIF" => safety: unreviewed();
        "AVERAGEIFS" => safety: unreviewed();
        "BAHTTEXT" => safety: unreviewed();
        "BESSELI" => safety: unreviewed();
        "BESSELJ" => safety: unreviewed();
        "BESSELK" => safety: unreviewed();
        "BESSELY" => safety: unreviewed();
        "BETADIST" => safety: unreviewed();
        "BETAINV" => safety: unreviewed();
        "BIN2DEC" => safety: unreviewed();
        "BIN2HEX" => safety: unreviewed();
        "BIN2OCT" => safety: unreviewed();
        "BINOMDIST" => safety: unreviewed();
        "CEILING" => safety: unreviewed();
        // CELL("filename") returns the full name of the file (ECMA-376-1:2016 §18.17.7.34).
        "CELL" => safety: dangerous(Danger::MachineInfo);
        "CHAR" => safety: unreviewed();
        "CHIDIST" => safety: unreviewed();
        "CHIINV" => safety: unreviewed();
        "CHITEST" => safety: unreviewed();
        "CHOOSE" => safety: unreviewed();
        "CLEAN" => safety: unreviewed();
        "CODE" => safety: unreviewed();
        "COLUMN" => safety: unreviewed();
        "COLUMNS" => safety: unreviewed();
        "COMBIN" => safety: unreviewed();
        "COMPLEX" => safety: unreviewed();
        "CONCATENATE" => safety: unreviewed();
        "CONFIDENCE" => safety: unreviewed();
        "CONVERT" => safety: unreviewed();
        "CORREL" => safety: unreviewed();
        "COS" => safety: unreviewed();
        "COSH" => safety: unreviewed();
        "COUNT" => safety: unreviewed();
        "COUNTA" => safety: unreviewed();
        "COUNTBLANK" => safety: unreviewed();
        "COUNTIF" => safety: unreviewed();
        "COUNTIFS" => safety: unreviewed();
        "COUPDAYBS" => safety: unreviewed();
        "COUPDAYS" => safety: unreviewed();
        "COUPDAYSNC" => safety: unreviewed();
        "COUPNCD" => safety: unreviewed();
        "COUPNUM" => safety: unreviewed();
        "COUPPCD" => safety: unreviewed();
        "COVAR" => safety: unreviewed();
        "CRITBINOM" => safety: unreviewed();
        "CUBEKPIMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBEMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBEMEMBERPROPERTY" => safety: dangerous(Danger::ExternalData);
        "CUBERANKEDMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBESET" => safety: dangerous(Danger::ExternalData);
        "CUBESETCOUNT" => safety: dangerous(Danger::ExternalData);
        "CUBEVALUE" => safety: dangerous(Danger::ExternalData);
        "CUMIPMT" => safety: unreviewed();
        "CUMPRINC" => safety: unreviewed();
        "DATE" => safety: unreviewed();
        "DATEDIF" => safety: unreviewed();
        "DATEVALUE" => safety: unreviewed();
        "DAVERAGE" => safety: unreviewed();
        "DAY" => safety: unreviewed();
        "DAYS360" => safety: unreviewed();
        "DB" => safety: unreviewed();
        "DCOUNT" => safety: unreviewed();
        "DCOUNTA" => safety: unreviewed();
        "DDB" => safety: unreviewed();
        "DEC2BIN" => safety: unreviewed();
        "DEC2HEX" => safety: unreviewed();
        "DEC2OCT" => safety: unreviewed();
        "DEGREES" => safety: unreviewed();
        "DELTA" => safety: unreviewed();
        "DEVSQ" => safety: unreviewed();
        "DGET" => safety: unreviewed();
        "DISC" => safety: unreviewed();
        "DMAX" => safety: unreviewed();
        "DMIN" => safety: unreviewed();
        "DOLLAR" => safety: unreviewed();
        "DOLLARDE" => safety: unreviewed();
        "DOLLARFR" => safety: unreviewed();
        "DPRODUCT" => safety: unreviewed();
        "DSTDEV" => safety: unreviewed();
        "DSTDEVP" => safety: unreviewed();
        "DSUM" => safety: unreviewed();
        "DURATION" => safety: unreviewed();
        "DVAR" => safety: unreviewed();
        "DVARP" => safety: unreviewed();
        "ECMA.CEILING" => safety: unreviewed();
        "EDATE" => safety: unreviewed();
        "EFFECT" => safety: unreviewed();
        "EOMONTH" => safety: unreviewed();
        "ERF" => safety: unreviewed();
        "ERFC" => safety: unreviewed();
        "ERROR.TYPE" => safety: unreviewed();
        "EVEN" => safety: unreviewed();
        "EXACT" => safety: unreviewed();
        "EXP" => safety: unreviewed();
        "EXPONDIST" => safety: unreviewed();
        "FACT" => safety: unreviewed();
        "FACTDOUBLE" => safety: unreviewed();
        "FALSE" => safety: unreviewed();
        "FDIST" => safety: unreviewed();
        "FIND" => safety: unreviewed();
        "FINDB" => safety: unreviewed();
        "FINV" => safety: unreviewed();
        "FISHER" => safety: unreviewed();
        "FISHERINV" => safety: unreviewed();
        "FIXED" => safety: unreviewed();
        "FLOOR" => safety: unreviewed();
        "FORECAST" => safety: unreviewed();
        "FREQUENCY" => safety: unreviewed();
        "FTEST" => safety: unreviewed();
        "FV" => safety: unreviewed();
        "FVSCHEDULE" => safety: unreviewed();
        "GAMMADIST" => safety: unreviewed();
        "GAMMAINV" => safety: unreviewed();
        "GAMMALN" => safety: unreviewed();
        "GCD" => safety: unreviewed();
        "GEOMEAN" => safety: unreviewed();
        "GESTEP" => safety: unreviewed();
        "GETPIVOTDATA" => safety: unreviewed();
        "GROWTH" => safety: unreviewed();
        "HARMEAN" => safety: unreviewed();
        "HEX2BIN" => safety: unreviewed();
        "HEX2DEC" => safety: unreviewed();
        "HEX2OCT" => safety: unreviewed();
        "HLOOKUP" => safety: unreviewed();
        "HOUR" => safety: unreviewed();
        "HYPERLINK" => safety: dangerous(Danger::OpensLink);
        "HYPGEOMDIST" => safety: unreviewed();
        "IF" => safety: unreviewed();
        "IFERROR" => safety: unreviewed();
        "IMABS" => safety: unreviewed();
        "IMAGINARY" => safety: unreviewed();
        "IMARGUMENT" => safety: unreviewed();
        "IMCONJUGATE" => safety: unreviewed();
        "IMCOS" => safety: unreviewed();
        "IMDIV" => safety: unreviewed();
        "IMEXP" => safety: unreviewed();
        "IMLN" => safety: unreviewed();
        "IMLOG10" => safety: unreviewed();
        "IMLOG2" => safety: unreviewed();
        "IMPOWER" => safety: unreviewed();
        "IMPRODUCT" => safety: unreviewed();
        "IMREAL" => safety: unreviewed();
        "IMSIN" => safety: unreviewed();
        "IMSQRT" => safety: unreviewed();
        "IMSUB" => safety: unreviewed();
        "IMSUM" => safety: unreviewed();
        "INDEX" => safety: unreviewed();
        "INDIRECT" => safety: unreviewed();
        // INFO("directory") and INFO("osversion") return the current directory and the operating
        // system's version (ECMA-376-1:2016 §18.17.7.168).
        "INFO" => safety: dangerous(Danger::MachineInfo);
        "INT" => safety: unreviewed();
        "INTERCEPT" => safety: unreviewed();
        "INTRATE" => safety: unreviewed();
        "IPMT" => safety: unreviewed();
        "IRR" => safety: unreviewed();
        "ISBLANK" => safety: unreviewed();
        "ISERR" => safety: unreviewed();
        "ISERROR" => safety: unreviewed();
        "ISEVEN" => safety: unreviewed();
        "ISLOGICAL" => safety: unreviewed();
        "ISNA" => safety: unreviewed();
        "ISNONTEXT" => safety: unreviewed();
        "ISNUMBER" => safety: unreviewed();
        "ISODD" => safety: unreviewed();
        "ISPMT" => safety: unreviewed();
        "ISREF" => safety: unreviewed();
        "ISTEXT" => safety: unreviewed();
        "JIS" => safety: unreviewed();
        "KURT" => safety: unreviewed();
        "LARGE" => safety: unreviewed();
        "LCM" => safety: unreviewed();
        "LEFT" => safety: unreviewed();
        "LEFTB" => safety: unreviewed();
        "LEN" => safety: unreviewed();
        "LENB" => safety: unreviewed();
        "LINEST" => safety: unreviewed();
        "LN" => safety: unreviewed();
        "LOG" => safety: unreviewed();
        "LOG10" => safety: unreviewed();
        "LOGEST" => safety: unreviewed();
        "LOGINV" => safety: unreviewed();
        "LOGNORMDIST" => safety: unreviewed();
        "LOOKUP" => safety: unreviewed();
        "LOWER" => safety: unreviewed();
        "MATCH" => safety: unreviewed();
        "MAX" => safety: unreviewed();
        "MAXA" => safety: unreviewed();
        "MDETERM" => safety: unreviewed();
        "MDURATION" => safety: unreviewed();
        "MEDIAN" => safety: unreviewed();
        "MID" => safety: unreviewed();
        "MIDB" => safety: unreviewed();
        "MIN" => safety: unreviewed();
        "MINA" => safety: unreviewed();
        "MINUTE" => safety: unreviewed();
        "MINVERSE" => safety: unreviewed();
        "MIRR" => safety: unreviewed();
        "MMULT" => safety: unreviewed();
        "MOD" => safety: unreviewed();
        "MODE" => safety: unreviewed();
        "MONTH" => safety: unreviewed();
        "MROUND" => safety: unreviewed();
        "MULTINOMIAL" => safety: unreviewed();
        "N" => safety: unreviewed();
        "NA" => safety: unreviewed();
        "NEGBINOMDIST" => safety: unreviewed();
        "NETWORKDAYS" => safety: unreviewed();
        "NOMINAL" => safety: unreviewed();
        "NORMDIST" => safety: unreviewed();
        "NORMINV" => safety: unreviewed();
        "NORMSDIST" => safety: unreviewed();
        "NORMSINV" => safety: unreviewed();
        "NOT" => safety: unreviewed();
        "NOW" => safety: unreviewed();
        "NPER" => safety: unreviewed();
        "NPV" => safety: unreviewed();
        "OCT2BIN" => safety: unreviewed();
        "OCT2DEC" => safety: unreviewed();
        "OCT2HEX" => safety: unreviewed();
        "ODD" => safety: unreviewed();
        "ODDFPRICE" => safety: unreviewed();
        "ODDFYIELD" => safety: unreviewed();
        "ODDLPRICE" => safety: unreviewed();
        "ODDLYIELD" => safety: unreviewed();
        "OFFSET" => safety: unreviewed();
        "OR" => safety: unreviewed();
        "PEARSON" => safety: unreviewed();
        "PERCENTILE" => safety: unreviewed();
        "PERCENTRANK" => safety: unreviewed();
        "PERMUT" => safety: unreviewed();
        "PHONETIC" => safety: unreviewed();
        "PI" => safety: unreviewed();
        "PMT" => safety: unreviewed();
        "POISSON" => safety: unreviewed();
        "POWER" => safety: unreviewed();
        "PPMT" => safety: unreviewed();
        "PRICE" => safety: unreviewed();
        "PRICEDISC" => safety: unreviewed();
        "PRICEMAT" => safety: unreviewed();
        "PROB" => safety: unreviewed();
        "PRODUCT" => safety: unreviewed();
        "PROPER" => safety: unreviewed();
        "PV" => safety: unreviewed();
        "QUARTILE" => safety: unreviewed();
        "QUOTIENT" => safety: unreviewed();
        "RADIANS" => safety: unreviewed();
        "RAND" => safety: unreviewed();
        "RANDBETWEEN" => safety: unreviewed();
        "RANK" => safety: unreviewed();
        "RATE" => safety: unreviewed();
        "RECEIVED" => safety: unreviewed();
        "REPLACE" => safety: unreviewed();
        "REPLACEB" => safety: unreviewed();
        "REPT" => safety: unreviewed();
        "RIGHT" => safety: unreviewed();
        "RIGHTB" => safety: unreviewed();
        "ROMAN" => safety: unreviewed();
        "ROUND" => safety: unreviewed();
        "ROUNDDOWN" => safety: unreviewed();
        "ROUNDUP" => safety: unreviewed();
        "ROW" => safety: unreviewed();
        "ROWS" => safety: unreviewed();
        "RSQ" => safety: unreviewed();
        "RTD" => safety: dangerous(Danger::ExternalData);
        "SEARCH" => safety: unreviewed();
        "SEARCHB" => safety: unreviewed();
        "SECOND" => safety: unreviewed();
        "SERIESSUM" => safety: unreviewed();
        "SIGN" => safety: unreviewed();
        "SIN" => safety: unreviewed();
        "SINH" => safety: unreviewed();
        "SKEW" => safety: unreviewed();
        "SLN" => safety: unreviewed();
        "SLOPE" => safety: unreviewed();
        "SMALL" => safety: unreviewed();
        "SQRT" => safety: unreviewed();
        "SQRTPI" => safety: unreviewed();
        "STANDARDIZE" => safety: unreviewed();
        "STDEV" => safety: unreviewed();
        "STDEVA" => safety: unreviewed();
        "STDEVP" => safety: unreviewed();
        "STDEVPA" => safety: unreviewed();
        "STEYX" => safety: unreviewed();
        "SUBSTITUTE" => safety: unreviewed();
        "SUBTOTAL" => safety: unreviewed();
        "SUM" => safety: unreviewed();
        "SUMIF" => safety: unreviewed();
        "SUMIFS" => safety: unreviewed();
        "SUMPRODUCT" => safety: unreviewed();
        "SUMSQ" => safety: unreviewed();
        "SUMX2MY2" => safety: unreviewed();
        "SUMX2PY2" => safety: unreviewed();
        "SUMXMY2" => safety: unreviewed();
        "SYD" => safety: unreviewed();
        "T" => safety: unreviewed();
        "TAN" => safety: unreviewed();
        "TANH" => safety: unreviewed();
        "TBILLEQ" => safety: unreviewed();
        "TBILLPRICE" => safety: unreviewed();
        "TBILLYIELD" => safety: unreviewed();
        "TDIST" => safety: unreviewed();
        "TEXT" => safety: unreviewed();
        "TIME" => safety: unreviewed();
        "TIMEVALUE" => safety: unreviewed();
        "TINV" => safety: unreviewed();
        "TODAY" => safety: unreviewed();
        "TRANSPOSE" => safety: unreviewed();
        "TREND" => safety: unreviewed();
        "TRIM" => safety: unreviewed();
        "TRIMMEAN" => safety: unreviewed();
        "TRUE" => safety: unreviewed();
        "TRUNC" => safety: unreviewed();
        "TTEST" => safety: unreviewed();
        "TYPE" => safety: unreviewed();
        "UPPER" => safety: unreviewed();
        "USDOLLAR" => safety: unreviewed();
        "VALUE" => safety: unreviewed();
        "VAR" => safety: unreviewed();
        "VARA" => safety: unreviewed();
        "VARP" => safety: unreviewed();
        "VARPA" => safety: unreviewed();
        "VDB" => safety: unreviewed();
        "VLOOKUP" => safety: unreviewed();
        "WEEKDAY" => safety: unreviewed();
        "WEEKNUM" => safety: unreviewed();
        "WEIBULL" => safety: unreviewed();
        "WORKDAY" => safety: unreviewed();
        "XIRR" => safety: unreviewed();
        "XNPV" => safety: unreviewed();
        "YEAR" => safety: unreviewed();
        "YEARFRAC" => safety: unreviewed();
        "YIELD" => safety: unreviewed();
        "YIELDDISC" => safety: unreviewed();
        "YIELDMAT" => safety: unreviewed();
        "ZTEST" => safety: unreviewed();
        // Not in ECMA-376. Their support columns are not filled. ANCHORARRAY
        // and SINGLE are the functions Excel writes for a spilled range
        // (`J1#` is saved as `_xlfn.ANCHORARRAY(J1)`) and for `@` in a
        // formula that also spills (`=A1:A10+@A1:A10` is saved with
        // `_xlfn.SINGLE(A1:A10)`, as the page on the implicit intersection
        // operator says); the dynamic_array feature's row covers their
        // versions. CALL and REGISTER.ID are older than every
        // version the table lists. COPILOT
        // was a preview function of Excel for Microsoft 365, which removed
        // it on 2026-09-14, as COPILOT's page says; a workbook saved before
        // that day may still call it, and verify is to deny it.

        "ANCHORARRAY" => safety: unreviewed();
        "SINGLE" => safety: unreviewed();
        // https://support.microsoft.com/en-us/office/call-function-32d58445-e646-4ffd-8d5e-b45077a5e995
        "CALL" => safety: dangerous(Danger::ExternalCode);
        // https://support.microsoft.com/en-us/excel/functions/copilot-function
        "COPILOT" => safety: dangerous(Danger::Network);
        // https://support.microsoft.com/en-us/excel/functions/register-id-function
        "REGISTER.ID" => safety: dangerous(Danger::ExternalCode);
    }
}
