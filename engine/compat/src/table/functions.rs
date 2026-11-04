//! The function rows: one per function Excel 2010 or later added that the
//! research of 2026-10-20 lists or [MS-XLSX] gives an `_xlfn.` prefix,
//! named without the `_xlfn.` / `_xlws.` prefix.
//!
//! A function Excel 2007 already had (`SUM`, `IFERROR`) has no row yet;
//! the table says nothing about it. The columns are excel, excel-mac,
//! libreoffice, google-sheets, numbers, onlyoffice. A cell no cited source
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
use super::{Feature, YES, between, le, no, on, s, unk, xl};
use crate::app::{
    Excel as X, ExcelMac as M, GoogleSheets as G, LibreOffice as L, Numbers as N, OnlyOffice as O,
};
use crate::entry::{Entry, Fallback::NameError};
use crate::row::{Row, Support};

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
    "BETA.DIST" => E2010, requires: [], sources: S2010;
    "BETA.INV" => E2010, requires: [], sources: S2010;
    "BINOM.DIST" => E2010, requires: [], sources: S2010;
    "BINOM.INV" => E2010, requires: [], sources: S2010;
    "CHISQ.DIST" => E2010, requires: [], sources: S2010;
    "CHISQ.DIST.RT" => E2010, requires: [], sources: S2010;
    "CHISQ.INV" => E2010, requires: [], sources: S2010;
    "CHISQ.INV.RT" => E2010, requires: [], sources: S2010;
    "CHISQ.TEST" => E2010, requires: [], sources: S2010;
    "CONFIDENCE.NORM" => E2010, requires: [], sources: S2010;
    "CONFIDENCE.T" => E2010, requires: [], sources: S2010;
    "COVARIANCE.P" => E2010, requires: [], sources: S2010;
    "COVARIANCE.S" => E2010, requires: [], sources: S2010;
    "ERF.PRECISE" => E2010, requires: [], sources: S2010;
    "ERFC.PRECISE" => E2010, requires: [], sources: S2010;
    "EXPON.DIST" => E2010, requires: [], sources: S2010;
    "F.DIST" => E2010, requires: [], sources: S2010;
    "F.DIST.RT" => E2010, requires: [], sources: S2010;
    "F.INV" => E2010, requires: [], sources: S2010;
    "F.INV.RT" => E2010, requires: [], sources: S2010;
    "F.TEST" => E2010, requires: [], sources: S2010;
    "GAMMA.DIST" => E2010, requires: [], sources: S2010;
    "GAMMA.INV" => E2010, requires: [], sources: S2010;
    "GAMMALN.PRECISE" => E2010, requires: [], sources: S2010;
    "LOGNORM.DIST" => E2010, requires: [], sources: S2010;
    "LOGNORM.INV" => E2010, requires: [], sources: S2010;
    "MODE.MULT" => E2010, requires: [], sources: S2010;
    "MODE.SNGL" => E2010, requires: [], sources: S2010;
    "NEGBINOM.DIST" => E2010, requires: [], sources: S2010;
    "NETWORKDAYS.INTL" => E2010, requires: [], sources: S2010;
    "NORM.DIST" => E2010, requires: [], sources: S2010;
    "NORM.INV" => E2010, requires: [], sources: S2010;
    "NORM.S.DIST" => E2010, requires: [], sources: S2010;
    "NORM.S.INV" => E2010, requires: [], sources: S2010;
    "PERCENTILE.EXC" => E2010, requires: [], sources: S2010;
    "PERCENTILE.INC" => E2010, requires: [], sources: S2010;
    "PERCENTRANK.EXC" => E2010, requires: [], sources: S2010;
    "PERCENTRANK.INC" => E2010, requires: [], sources: S2010;
    "POISSON.DIST" => E2010, requires: [], sources: S2010;
    "QUARTILE.EXC" => E2010, requires: [], sources: S2010;
    "QUARTILE.INC" => E2010, requires: [], sources: S2010;
    "RANK.AVG" => E2010, requires: [], sources: S2010;
    "RANK.EQ" => E2010, requires: [], sources: S2010;
    "STDEV.P" => E2010, requires: [], sources: S2010;
    "STDEV.S" => E2010, requires: [], sources: S2010;
    "T.DIST" => E2010, requires: [], sources: S2010;
    "T.DIST.2T" => E2010, requires: [], sources: S2010;
    "T.DIST.RT" => E2010, requires: [], sources: S2010;
    "T.INV" => E2010, requires: [], sources: S2010;
    "T.INV.2T" => E2010, requires: [], sources: S2010;
    "T.TEST" => E2010, requires: [], sources: S2010;
    "VAR.P" => E2010, requires: [], sources: S2010;
    "VAR.S" => E2010, requires: [], sources: S2010;
    "WEIBULL.DIST" => E2010, requires: [], sources: S2010;
    "WORKDAY.INTL" => E2010, requires: [], sources: S2010;
    "Z.TEST" => E2010, requires: [], sources: S2010;
    // No version in the functions by category list; AGGREGATE's own page starts at 2016.
    "AGGREGATE" => s(between(X::V2007, X::V2016).fallback(NameError), le(M::V2021), on(L::V24_2), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_AGGREGATE, MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC];
    "ARABIC" => E_UNDATED, requires: [], sources: S_UNDATED;
    "CEILING.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED;
    "FLOOR.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED;
    "HYPGEOM.DIST" => E_UNDATED, requires: [], sources: S_UNDATED;
    // Excel 2013.
    "ACOT" => E2013, requires: [], sources: S2013;
    "ACOTH" => E2013, requires: [], sources: S2013;
    "BASE" => E2013, requires: [], sources: S2013;
    "BINOM.DIST.RANGE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013;
    "BITAND" => E2013, requires: [], sources: S2013;
    "BITLSHIFT" => E2013, requires: [], sources: S2013;
    "BITOR" => E2013, requires: [], sources: S2013;
    "BITRSHIFT" => E2013, requires: [], sources: S2013;
    "BITXOR" => E2013, requires: [], sources: S2013;
    "CEILING.MATH" => E2013, requires: [], sources: S2013;
    "COMBINA" => E2013, requires: [], sources: S2013;
    "COT" => E2013, requires: [], sources: S2013;
    "COTH" => E2013, requires: [], sources: S2013;
    "CSC" => E2013, requires: [], sources: S2013;
    "CSCH" => E2013, requires: [], sources: S2013;
    "DAYS" => E2013, requires: [], sources: S2013;
    "DECIMAL" => E2013, requires: [], sources: S2013;
    "ENCODEURL" => E2013.with_excel_mac(MAC_WINDOWS_ONLY), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_ENCODEURL, LO_HELP, GS_LIST, OO_CL, OO_SRC];
    "FILTERXML" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_FILTERXML, LO_HELP, GS_LIST, OO_CL, OO_SRC];
    "FLOOR.MATH" => E2013, requires: [], sources: S2013;
    "FORMULATEXT" => E2013, requires: [], sources: S2013;
    "GAMMA" => E2013, requires: [], sources: S2013;
    "GAUSS" => E2013, requires: [], sources: S2013;
    "IFNA" => E2013, requires: [], sources: S2013;
    "IMCOSH" => E2013, requires: [], sources: S2013;
    "IMCOT" => E2013, requires: [], sources: S2013;
    "IMCSC" => E2013, requires: [], sources: S2013;
    "IMCSCH" => E2013, requires: [], sources: S2013;
    "IMSEC" => E2013, requires: [], sources: S2013;
    "IMSECH" => E2013, requires: [], sources: S2013;
    "IMSINH" => E2013, requires: [], sources: S2013;
    "IMTAN" => E2013, requires: [], sources: S2013;
    "ISFORMULA" => E2013, requires: [], sources: S2013;
    "ISO.CEILING" => E2013, requires: [], sources: S2013;
    "ISOWEEKNUM" => E2013, requires: [], sources: S2013;
    "MUNIT" => E2013, requires: [], sources: S2013;
    "NUMBERVALUE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013;
    "PDURATION" => E2013, requires: [], sources: S2013;
    "PERMUTATIONA" => E2013, requires: [], sources: S2013;
    "PHI" => E2013, requires: [], sources: S2013;
    "RRI" => E2013, requires: [], sources: S2013;
    "SEC" => E2013, requires: [], sources: S2013;
    "SECH" => E2013, requires: [], sources: S2013;
    "SHEET" => E2013, requires: [], sources: S2013;
    "SHEETS" => E2013, requires: [], sources: S2013;
    "SKEW.P" => E2013, requires: [], sources: S2013;
    "UNICHAR" => E2013, requires: [], sources: S2013;
    "UNICODE" => E2013, requires: [], sources: S2013;
    "WEBSERVICE" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_WEBSERVICE, LO_HELP, GS_LIST, OO_CL, OO_SRC];
    "XOR" => E2013, requires: [], sources: S2013;
    // Excel 2016.
    "FORECAST.ETS" => E2016, requires: [], sources: S2016;
    "FORECAST.ETS.CONFINT" => E2016, requires: [], sources: S2016;
    "FORECAST.ETS.SEASONALITY" => E2016, requires: [], sources: S2016;
    "FORECAST.ETS.STAT" => E2016, requires: [], sources: S2016;
    "FORECAST.LINEAR" => E2016.with_google_sheets(YES), requires: [], sources: S2016;
    // Excel 2019.
    "CONCAT" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), le(O::V10_0)), requires: [], sources: S2019;
    "IFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019;
    "MAXIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019;
    "MINIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019;
    "SWITCH" => s(xl(X::V2019).note(SWITCH_2016), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_SWITCH, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_FEATURES, OO_CL];
    "TEXTJOIN" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: S2019;
    // Excel 2021.
    "ARRAYTOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, OO_CL];
    "FILTER" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021;
    "LET" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), unk()), requires: [], sources: S2021;
    "RANDARRAY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021;
    "SEQUENCE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021;
    "SORT" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021;
    "SORTBY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), unk(), unk(), le(O::V10_0)), requires: [DynamicArray], sources: S2021;
    "UNIQUE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021;
    "VALUETOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN];
    "XLOOKUP" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_ICLOUD, NUM_XLOOKUP, OO_CL];
    "XMATCH" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, LO_HELP, GS_XMATCH, NUM_ICLOUD, NUM_XMATCH, OO_CL];
    // Excel 2024.
    "BYCOL" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA;
    "BYROW" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA;
    "CHOOSECOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "CHOOSEROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "DROP" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024;
    "EXPAND" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024;
    "HSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "IMAGE" => s(xl(X::V2024), xl(M::V2024), no(), YES.partial("its arguments differ from Excel's"), unk(), unk()), requires: [], sources: &[MS_FN, MS_IMAGE, MS_NEW_2024, MS_XLFN, LO_HELP, GS_LIST];
    "ISOMITTED" => s(xl(X::V2024), xl(M::V2024), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2024, MS_XLFN];
    "LAMBDA" => E2024_LAMBDA.with_libreoffice(no().note("a workbook with LAMBDA is reported to show Err:508")).with_numbers(on(N::V14_4)), requires: [], sources: S2024_LAMBDA;
    "MAKEARRAY" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA;
    "MAP" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA;
    "REDUCE" => E2024_LAMBDA, requires: [], sources: S2024_LAMBDA;
    "SCAN" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA;
    "TAKE" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024;
    "TEXTAFTER" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024;
    "TEXTBEFORE" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024;
    "TEXTSPLIT" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024;
    "TOCOL" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "TOROW" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "VSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "WRAPCOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    "WRAPROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024;
    // Microsoft 365 only.
    "DETECTLANGUAGE" => E365_ONLY.with_google_sheets(YES.note("a function of the name is in the list; its results were not compared with Excel's")), requires: [], sources: &[MS_FN, MS_XLFN, GS_LIST];
    "FIELDVALUE" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN];
    "GROUPBY" => E365, requires: [DynamicArray], sources: S365;
    "PERCENTOF" => E365, requires: [], sources: S365;
    "PIVOTBY" => E365, requires: [DynamicArray], sources: S365;
    "PY" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN];
    "REGEXEXTRACT" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX;
    "REGEXREPLACE" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX;
    "REGEXTEST" => E365.with_libreoffice(LO_OWN_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX;
    "STOCKHISTORY" => E365_ONLY, requires: [DynamicArray], sources: &[MS_FN, MS_XLFN];
    "TRANSLATE" => E365_ONLY, requires: [], sources: &[MS_FN, MS_XLFN];
    "TRIMRANGE" => E365, requires: [DynamicArray], sources: S365;
}
