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
//! The dangerous functions are those that use the network (WEBSERVICE,
//! IMAGE, STOCKHISTORY, TRANSLATE, DETECTLANGUAGE, PY, COPILOT), read a
//! source outside the workbook (RTD and the CUBE functions), run code
//! outside the workbook (CALL, REGISTER.ID), return information about the
//! machine or the workbook's file (INFO, CELL) or open a link
//! (HYPERLINK).
//!
//! Every function but the dangerous ones and the unreviewed ones below is
//! `reviewed(sources)`: on [`REVIEWED_ON`](super::REVIEWED_ON), its
//! definition was read, and the definition says its result depends only on
//! what [`crate::Safety::ReviewedSafe`] lists. A function ECMA-376-1:2016
//! §18.17.7 defines cites its own section; any other function cites its
//! page at Microsoft, or for SINGLE the page on the operator Excel saves
//! as SINGLE. Eight functions stay `unreviewed()`, each with the reason in
//! a comment above its row: seven (CHAR, CODE, FORMULATEXT, FILTERXML,
//! FIELDVALUE, GETPIVOTDATA and ANCHORARRAY) whose definitions leave in
//! doubt whether the result depends only on that list, or which no
//! definition was found for, and ENCODEURL, whose page gives encoding the
//! URL WEBSERVICE requests as its only use.
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
use super::{Feature, YES, between, dangerous, le, no, on, reviewed, s, unk, unreviewed, xl};
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
    "BETA.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/beta-dist-function"]);
    "BETA.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/beta-inv-function"]);
    "BINOM.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/binom-dist-function"]);
    "BINOM.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/binom-inv-function"]);
    "CHISQ.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chisq-dist-function"]);
    "CHISQ.DIST.RT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chisq-dist-rt-function"]);
    "CHISQ.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chisq-inv-function"]);
    "CHISQ.INV.RT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chisq-inv-rt-function"]);
    "CHISQ.TEST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chisq-test-function"]);
    "CONFIDENCE.NORM" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/confidence-norm-function"]);
    "CONFIDENCE.T" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/confidence-t-function"]);
    "COVARIANCE.P" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/covariance-p-function"]);
    "COVARIANCE.S" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/covariance-s-function"]);
    "ERF.PRECISE" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/erf-precise-function"]);
    "ERFC.PRECISE" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/erfc-precise-function"]);
    "EXPON.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/expon-dist-function"]);
    "F.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/f-dist-function"]);
    "F.DIST.RT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/f-dist-rt-function"]);
    "F.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/f-inv-function"]);
    "F.INV.RT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/f-inv-rt-function"]);
    "F.TEST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/f-test-function"]);
    "GAMMA.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/gamma-dist-function"]);
    "GAMMA.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/gamma-inv-function"]);
    "GAMMALN.PRECISE" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/gammaln-precise-function"]);
    "LOGNORM.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/lognorm-dist-function"]);
    "LOGNORM.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/lognorm-inv-function"]);
    "MODE.MULT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/mode-mult-function"]);
    "MODE.SNGL" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/mode-sngl-function"]);
    "NEGBINOM.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/negbinom-dist-function"]);
    "NETWORKDAYS.INTL" => E2010, requires: [], sources: S2010, safety: reviewed(&["ECMA-376-1:2016 §18.17.7.227"]);
    "NORM.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/norm-dist-function"]);
    "NORM.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/norm-inv-function"]);
    "NORM.S.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/norm-s-dist-function"]);
    "NORM.S.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/norm-s-inv-function"]);
    "PERCENTILE.EXC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/percentile-exc-function"]);
    "PERCENTILE.INC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/percentile-inc-function"]);
    "PERCENTRANK.EXC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/percentrank-exc-function"]);
    "PERCENTRANK.INC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/percentrank-inc-function"]);
    "POISSON.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/poisson-dist-function"]);
    "QUARTILE.EXC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/quartile-exc-function"]);
    "QUARTILE.INC" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/quartile-inc-function"]);
    "RANK.AVG" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/rank-avg-function"]);
    "RANK.EQ" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/rank-eq-function"]);
    "STDEV.P" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/stdev-p-function"]);
    "STDEV.S" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/stdev-s-function"]);
    "T.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-dist-function"]);
    "T.DIST.2T" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-dist-2t-function"]);
    "T.DIST.RT" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-dist-rt-function"]);
    "T.INV" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-inv-function"]);
    "T.INV.2T" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-inv-2t-function"]);
    "T.TEST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/t-test-function"]);
    "VAR.P" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/var-p-function"]);
    "VAR.S" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/var-s-function"]);
    "WEIBULL.DIST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/weibull-dist-function"]);
    "WORKDAY.INTL" => E2010, requires: [], sources: S2010, safety: reviewed(&["ECMA-376-1:2016 §18.17.7.348"]);
    "Z.TEST" => E2010, requires: [], sources: S2010, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/z-test-function"]);
    // No version in the functions by category list; AGGREGATE's own page starts at 2016.
    "AGGREGATE" => s(between(X::V2007, X::V2016).fallback(NameError), le(M::V2021), on(L::V24_2), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_AGGREGATE, MS_FN, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: reviewed(&[MS_AGGREGATE]);
    "ARABIC" => E_UNDATED, requires: [], sources: S_UNDATED, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/arabic-function"]);
    "CEILING.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/ceiling-precise-function"]);
    "FLOOR.PRECISE" => E_UNDATED, requires: [], sources: S_UNDATED, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/floor-precise-function"]);
    "HYPGEOM.DIST" => E_UNDATED, requires: [], sources: S_UNDATED, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/hypgeom-dist-function"]);
    // Excel 2013.
    "ACOT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/acot-function"]);
    "ACOTH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/acoth-function"]);
    "BASE" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/base-function"]);
    "BINOM.DIST.RANGE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/binom-dist-range-function"]);
    "BITAND" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bitand-function"]);
    "BITLSHIFT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bitlshift-function"]);
    "BITOR" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bitor-function"]);
    "BITRSHIFT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bitrshift-function"]);
    "BITXOR" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bitxor-function"]);
    "CEILING.MATH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/ceiling-math-function"]);
    "COMBINA" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/combina-function"]);
    "COT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/cot-function"]);
    "COTH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/coth-function"]);
    "CSC" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/csc-function"]);
    "CSCH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/csch-function"]);
    "DAYS" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/days-function"]);
    "DECIMAL" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/decimal-function"]);
    // Unreviewed: its result depends on its argument only, but its page says
    // ENCODEURL encodes the text of the URL that WEBSERVICE requests, and
    // WEBSERVICE is Danger::Network. A function whose only use its page gives
    // is to build an argument of a dangerous function stays unreviewed.
    "ENCODEURL" => E2013.with_excel_mac(MAC_WINDOWS_ONLY), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_ENCODEURL, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: unreviewed();
    // Unreviewed: its page says it relies on features of the Windows operating
    // system to read the XML, and does not say whether that reading fetches a
    // DTD or an external entity the XML names.
    "FILTERXML" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_FILTERXML, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: unreviewed();
    "FLOOR.MATH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/floor-math-function"]);
    // Unreviewed: it returns the formula bar's text of the cell it reads, as
    // its page says (https://support.microsoft.com/en-us/excel/functions/formulatext-function).
    // When that formula refers to a workbook that is not open, the formula
    // bar's text may hold the path of that workbook's file, which names a
    // folder of the machine as CELL("filename") does.
    "FORMULATEXT" => E2013, requires: [], sources: S2013, safety: unreviewed();
    "GAMMA" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/gamma-function"]);
    "GAUSS" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/gauss-function"]);
    "IFNA" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/ifna-function"]);
    "IMCOSH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imcosh-function"]);
    "IMCOT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imcot-function"]);
    "IMCSC" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imcsc-function"]);
    "IMCSCH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imcsch-function"]);
    "IMSEC" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imsec-function"]);
    "IMSECH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imsech-function"]);
    "IMSINH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imsinh-function"]);
    "IMTAN" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/imtan-function"]);
    "ISFORMULA" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/isformula-function"]);
    "ISO.CEILING" => E2013, requires: [], sources: S2013, safety: reviewed(&["ECMA-376-1:2016 §18.17.7.182"]);
    "ISOWEEKNUM" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/isoweeknum-function"]);
    "MUNIT" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/munit-function"]);
    "NUMBERVALUE" => E2013.with_google_sheets(unk()), requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/numbervalue-function"]);
    "PDURATION" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/pduration-function"]);
    "PERMUTATIONA" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/permutationa-function"]);
    "PHI" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/phi-function"]);
    "RRI" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/rri-function"]);
    "SEC" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sec-function"]);
    "SECH" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sech-function"]);
    "SHEET" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sheet-function"]);
    "SHEETS" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sheets-function"]);
    "SKEW.P" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/skew-p-function"]);
    "UNICHAR" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/unichar-function"]);
    "UNICODE" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/unicode-function"]);
    "WEBSERVICE" => E2013.with_excel_mac(MAC_WINDOWS_ONLY).with_google_sheets(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN, MS_WEBSERVICE, LO_HELP, GS_LIST, OO_CL, OO_SRC], safety: dangerous(Danger::Network);
    "XOR" => E2013, requires: [], sources: S2013, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/xor-function"]);
    // Excel 2016.
    "FORECAST.ETS" => E2016, requires: [], sources: S2016, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/forecast-ets-function"]);
    "FORECAST.ETS.CONFINT" => E2016, requires: [], sources: S2016, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/forecast-ets-confint-function"]);
    "FORECAST.ETS.SEASONALITY" => E2016, requires: [], sources: S2016, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/forecast-ets-seasonality-function"]);
    "FORECAST.ETS.STAT" => E2016, requires: [], sources: S2016, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/forecast-ets-stat-function"]);
    "FORECAST.LINEAR" => E2016.with_google_sheets(YES), requires: [], sources: S2016, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/forecast-and-forecast-linear-functions"]);
    // Excel 2019.
    "CONCAT" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), le(O::V10_0)), requires: [], sources: S2019, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/concat-function"]);
    "IFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/ifs-function"]);
    "MAXIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/maxifs-function"]);
    "MINIFS" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, unk(), le(O::V10_0)), requires: [], sources: S2019, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/minifs-function"]);
    "SWITCH" => s(xl(X::V2019).note(SWITCH_2016), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_SWITCH, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_FEATURES, OO_CL], safety: reviewed(&[MS_SWITCH]);
    "TEXTJOIN" => s(xl(X::V2019), xl(M::V2019), on(L::V24_2), YES, le(N::V14_4), on(O::V8_0)), requires: [], sources: S2019, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/textjoin-function"]);
    // Excel 2021.
    "ARRAYTOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, OO_CL], safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/arraytotext-function"]);
    "FILTER" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/filter-function"]);
    "LET" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), unk()), requires: [], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/let-function"]);
    "RANDARRAY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/randarray-function"]);
    "SEQUENCE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, unk(), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sequence-function"]);
    "SORT" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sort-function"]);
    "SORTBY" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), unk(), unk(), le(O::V10_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/sortby-function"]);
    "UNIQUE" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, on(N::V14_4), on(O::V8_0)), requires: [DynamicArray], sources: S2021, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/unique-function"]);
    "VALUETOTEXT" => s(xl(X::V2021), xl(M::V2021), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN], safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/valuetotext-function"]);
    "XLOOKUP" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLSX_FN, MS_XLFN, LO_HELP, GS_LIST, NUM_ICLOUD, NUM_XLOOKUP, OO_CL], safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/xlookup-function"]);
    "XMATCH" => s(xl(X::V2021), xl(M::V2021), on(L::V24_8), YES, le(N::V14_4).note(NUMBERS_ICLOUD), on(O::V8_0)), requires: [], sources: &[MS_FN, MS_NEW_2021, MS_XLFN, LO_HELP, GS_XMATCH, NUM_ICLOUD, NUM_XMATCH, OO_CL], safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/xmatch-function"]);
    // Excel 2024.
    "BYCOL" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/bycol-function"]);
    "BYROW" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/byrow-function"]);
    "CHOOSECOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/choosecols-function"]);
    "CHOOSEROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/chooserows-function"]);
    "DROP" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/drop-function"]);
    "EXPAND" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/expand-function"]);
    "HSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/hstack-function"]);
    "IMAGE" => s(xl(X::V2024), xl(M::V2024), no(), YES.partial("its arguments differ from Excel's"), unk(), unk()), requires: [], sources: &[MS_FN, MS_IMAGE, MS_NEW_2024, MS_XLFN, LO_HELP, GS_LIST], safety: dangerous(Danger::Network);
    "ISOMITTED" => s(xl(X::V2024), xl(M::V2024), unk(), unk(), unk(), unk()), requires: [], sources: &[MS_FN, MS_NEW_2024, MS_XLFN], safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/isomitted-function"]);
    "LAMBDA" => E2024_LAMBDA.with_libreoffice(no().note("a workbook with LAMBDA is reported to show Err:508")).with_numbers(on(N::V14_4)), requires: [], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/lambda-function"]);
    "MAKEARRAY" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/makearray-function"]);
    "MAP" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/map-function"]);
    "REDUCE" => E2024_LAMBDA, requires: [], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/reduce-function"]);
    "SCAN" => E2024_LAMBDA, requires: [DynamicArray], sources: S2024_LAMBDA, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/scan-function"]);
    "TAKE" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/take-function"]);
    "TEXTAFTER" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/textafter-function"]);
    "TEXTBEFORE" => E2024_ARRAY.with_google_sheets(unk()), requires: [], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/textbefore-function"]);
    "TEXTSPLIT" => E2024_ARRAY.with_google_sheets(unk()), requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/textsplit-function"]);
    "TOCOL" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/tocol-function"]);
    "TOROW" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/torow-function"]);
    "VSTACK" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/vstack-function"]);
    "WRAPCOLS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/wrapcols-function"]);
    "WRAPROWS" => E2024_ARRAY, requires: [DynamicArray], sources: S2024, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/wraprows-function"]);
    // Microsoft 365 only.
    "DETECTLANGUAGE" => E365_ONLY.with_google_sheets(YES.note("a function of the name is in the list; its results were not compared with Excel's")), requires: [], sources: &[MS_FN, MS_XLFN, GS_LIST], safety: dangerous(Danger::Network);
    // Unreviewed: it returns a field of a linked data type (Stocks, Geography),
    // as its page says (https://support.microsoft.com/en-us/excel/functions/fieldvalue-function);
    // the page does not say whether Excel fetches the field from the online
    // source of the linked data type when it computes the formula.
    "FIELDVALUE" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN], safety: unreviewed();
    "GROUPBY" => E365, requires: [DynamicArray], sources: S365, safety: reviewed(&[MS_GROUPBY]);
    "PERCENTOF" => E365, requires: [], sources: S365, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/percentof-function"]);
    "PIVOTBY" => E365, requires: [DynamicArray], sources: S365, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/pivotby-function"]);
    "PY" => E365_ONLY.with_excel_mac(unk()), requires: [], sources: &[MS_FN, MS_XLSX_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "REGEXEXTRACT" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: reviewed(&[MS_REGEXEXTRACT]);
    "REGEXREPLACE" => E365.with_libreoffice(LO_OWN_REGEX).with_google_sheets(GS_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/regexreplace-function"]);
    "REGEXTEST" => E365.with_libreoffice(LO_OWN_REGEX).with_onlyoffice(on(O::V9_3)), requires: [], sources: S365_REGEX, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/regextest-function"]);
    "STOCKHISTORY" => E365_ONLY, requires: [DynamicArray], sources: &[MS_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "TRANSLATE" => E365_ONLY, requires: [], sources: &[MS_FN, MS_XLFN], safety: dangerous(Danger::Network);
    "TRIMRANGE" => E365, requires: [DynamicArray], sources: S365, safety: reviewed(&["https://support.microsoft.com/en-us/excel/functions/trimrange-function"]);
    unchecked {
        // The functions of ECMA-376-1:2016 §18.17.7 in its order, but for
        // ISO.CEILING, NETWORKDAYS.INTL and WORKDAY.INTL, which Excel added
        // later: their rows are in the Excel 2010 and Excel 2013 groups.
        // Which Excel first had each function of this block is not checked
        // yet.
        "ABS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.1"]);
        "ACCRINT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.2"]);
        "ACCRINTM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.3"]);
        "ACOS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.4"]);
        "ACOSH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.5"]);
        "ADDRESS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.6"]);
        "AMORDEGRC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.7"]);
        "AMORLINC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.8"]);
        "AND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.9"]);
        "AREAS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.10"]);
        "ASC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.11"]);
        "ASIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.12"]);
        "ASINH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.13"]);
        "ATAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.14"]);
        "ATAN2" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.15"]);
        "ATANH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.16"]);
        "AVEDEV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.17"]);
        "AVERAGE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.18"]);
        "AVERAGEA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.19"]);
        "AVERAGEIF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.20"]);
        "AVERAGEIFS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.21"]);
        "BAHTTEXT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.22"]);
        "BESSELI" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.23"]);
        "BESSELJ" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.24"]);
        "BESSELK" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.25"]);
        "BESSELY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.26"]);
        "BETADIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.27"]);
        "BETAINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.28"]);
        "BIN2DEC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.29"]);
        "BIN2HEX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.30"]);
        "BIN2OCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.31"]);
        "BINOMDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.32"]);
        "CEILING" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.33"]);
        // CELL("filename") returns the full name of the file (ECMA-376-1:2016 §18.17.7.34).
        "CELL" => safety: dangerous(Danger::MachineInfo);
        // Unreviewed: CHAR uses the Macintosh character set on the Macintosh
        // and ISO-8859-1 on every other platform (ECMA-376-1:2016
        // §18.17.7.35), and CODE's page at Microsoft
        // (https://support.microsoft.com/en-us/excel/functions/code-function)
        // says its result follows the character set of the computer, Macintosh
        // or Windows ANSI. The results of CHAR and CODE therefore tell whether
        // the machine that computes them is a Macintosh, as INFO("system")
        // does. UNICHAR and UNICODE, whose results do not depend on the
        // platform, are reviewed safe.
        "CHAR" => safety: unreviewed();
        "CHIDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.36"]);
        "CHIINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.37"]);
        "CHITEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.38"]);
        "CHOOSE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.39"]);
        "CLEAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.40"]);
        // Unreviewed: see the comment above CHAR.
        "CODE" => safety: unreviewed();
        "COLUMN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.42"]);
        "COLUMNS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.43"]);
        "COMBIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.44"]);
        "COMPLEX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.45"]);
        "CONCATENATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.46"]);
        "CONFIDENCE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.47"]);
        "CONVERT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.48"]);
        "CORREL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.49"]);
        "COS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.50"]);
        "COSH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.51"]);
        "COUNT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.52"]);
        "COUNTA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.53"]);
        "COUNTBLANK" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.54"]);
        "COUNTIF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.55"]);
        "COUNTIFS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.56"]);
        "COUPDAYBS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.57"]);
        "COUPDAYS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.58"]);
        "COUPDAYSNC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.59"]);
        "COUPNCD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.60"]);
        "COUPNUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.61"]);
        "COUPPCD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.62"]);
        "COVAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.63"]);
        "CRITBINOM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.64"]);
        "CUBEKPIMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBEMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBEMEMBERPROPERTY" => safety: dangerous(Danger::ExternalData);
        "CUBERANKEDMEMBER" => safety: dangerous(Danger::ExternalData);
        "CUBESET" => safety: dangerous(Danger::ExternalData);
        "CUBESETCOUNT" => safety: dangerous(Danger::ExternalData);
        "CUBEVALUE" => safety: dangerous(Danger::ExternalData);
        "CUMIPMT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.72"]);
        "CUMPRINC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.73"]);
        "DATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.74"]);
        "DATEDIF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.75"]);
        "DATEVALUE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.76"]);
        "DAVERAGE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.77"]);
        "DAY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.78"]);
        "DAYS360" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.79"]);
        "DB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.80"]);
        "DCOUNT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.81"]);
        "DCOUNTA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.82"]);
        "DDB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.83"]);
        "DEC2BIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.84"]);
        "DEC2HEX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.85"]);
        "DEC2OCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.86"]);
        "DEGREES" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.87"]);
        "DELTA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.88"]);
        "DEVSQ" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.89"]);
        "DGET" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.90"]);
        "DISC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.91"]);
        "DMAX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.92"]);
        "DMIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.93"]);
        "DOLLAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.94"]);
        "DOLLARDE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.95"]);
        "DOLLARFR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.96"]);
        "DPRODUCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.97"]);
        "DSTDEV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.98"]);
        "DSTDEVP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.99"]);
        "DSUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.100"]);
        "DURATION" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.101"]);
        "DVAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.102"]);
        "DVARP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.103"]);
        "ECMA.CEILING" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.104"]);
        "EDATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.105"]);
        "EFFECT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.106"]);
        "EOMONTH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.107"]);
        "ERF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.108"]);
        "ERFC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.109"]);
        "ERROR.TYPE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.110"]);
        "EVEN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.111"]);
        "EXACT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.112"]);
        "EXP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.113"]);
        "EXPONDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.114"]);
        "FACT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.115"]);
        "FACTDOUBLE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.116"]);
        "FALSE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.117"]);
        "FDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.118"]);
        "FIND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.119"]);
        "FINDB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.120"]);
        "FINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.121"]);
        "FISHER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.122"]);
        "FISHERINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.123"]);
        "FIXED" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.124"]);
        "FLOOR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.125"]);
        "FORECAST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.126"]);
        "FREQUENCY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.127"]);
        "FTEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.128"]);
        "FV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.129"]);
        "FVSCHEDULE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.130"]);
        "GAMMADIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.131"]);
        "GAMMAINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.132"]);
        "GAMMALN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.133"]);
        "GCD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.134"]);
        "GEOMEAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.135"]);
        "GESTEP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.136"]);
        // Unreviewed: ECMA-376-1:2016 §18.17.7.137 says its field and item
        // arguments may name the dimensions of an OLAP PivotTable report. Such a
        // report shows the data of an OLAP server, as the CUBE functions do;
        // the section does not say whether GETPIVOTDATA reads that data from
        // the server.
        "GETPIVOTDATA" => safety: unreviewed();
        "GROWTH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.138"]);
        "HARMEAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.139"]);
        "HEX2BIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.140"]);
        "HEX2DEC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.141"]);
        "HEX2OCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.142"]);
        "HLOOKUP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.143"]);
        "HOUR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.144"]);
        "HYPERLINK" => safety: dangerous(Danger::OpensLink);
        "HYPGEOMDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.146"]);
        "IF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.147"]);
        "IFERROR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.148"]);
        "IMABS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.149"]);
        "IMAGINARY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.150"]);
        "IMARGUMENT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.151"]);
        "IMCONJUGATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.152"]);
        "IMCOS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.153"]);
        "IMDIV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.154"]);
        "IMEXP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.155"]);
        "IMLN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.156"]);
        "IMLOG10" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.157"]);
        "IMLOG2" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.158"]);
        "IMPOWER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.159"]);
        "IMPRODUCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.160"]);
        "IMREAL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.161"]);
        "IMSIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.162"]);
        "IMSQRT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.163"]);
        "IMSUB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.164"]);
        "IMSUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.165"]);
        "INDEX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.166"]);
        // INDIRECT and OFFSET compute from their arguments the reference whose
        // cells they read. INDIRECT's ref-text may name the cells of another
        // workbook that is open (ECMA-376-1:2016 §18.17.7.167), which the
        // cells listed by Safety::ReviewedSafe do not include. Issue #15
        // accepts both functions, and the last pull request of issue #15 makes
        // verify report a notice that the cells INDIRECT and OFFSET read
        // cannot be known before Excel computes the formula.
        "INDIRECT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.167"]);
        // INFO("directory") and INFO("osversion") return the current directory and the operating
        // system's version (ECMA-376-1:2016 §18.17.7.168).
        "INFO" => safety: dangerous(Danger::MachineInfo);
        "INT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.169"]);
        "INTERCEPT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.170"]);
        "INTRATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.171"]);
        "IPMT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.172"]);
        "IRR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.173"]);
        "ISBLANK" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.174"]);
        "ISERR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.175"]);
        "ISERROR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.176"]);
        "ISEVEN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.177"]);
        "ISLOGICAL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.178"]);
        "ISNA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.179"]);
        "ISNONTEXT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.180"]);
        "ISNUMBER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.181"]);
        "ISODD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.183"]);
        "ISPMT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.184"]);
        "ISREF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.185"]);
        "ISTEXT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.186"]);
        "JIS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.187"]);
        "KURT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.188"]);
        "LARGE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.189"]);
        "LCM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.190"]);
        "LEFT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.191"]);
        "LEFTB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.192"]);
        "LEN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.193"]);
        "LENB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.194"]);
        "LINEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.195"]);
        "LN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.196"]);
        "LOG" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.197"]);
        "LOG10" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.198"]);
        "LOGEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.199"]);
        "LOGINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.200"]);
        "LOGNORMDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.201"]);
        "LOOKUP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.202"]);
        "LOWER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.203"]);
        "MATCH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.204"]);
        "MAX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.205"]);
        "MAXA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.206"]);
        "MDETERM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.207"]);
        "MDURATION" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.208"]);
        "MEDIAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.209"]);
        "MID" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.210"]);
        "MIDB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.211"]);
        "MIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.212"]);
        "MINA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.213"]);
        "MINUTE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.214"]);
        "MINVERSE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.215"]);
        "MIRR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.216"]);
        "MMULT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.217"]);
        "MOD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.218"]);
        "MODE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.219"]);
        "MONTH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.220"]);
        "MROUND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.221"]);
        "MULTINOMIAL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.222"]);
        "N" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.223"]);
        "NA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.224"]);
        "NEGBINOMDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.225"]);
        "NETWORKDAYS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.226"]);
        "NOMINAL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.228"]);
        "NORMDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.229"]);
        "NORMINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.230"]);
        "NORMSDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.231"]);
        "NORMSINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.232"]);
        "NOT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.233"]);
        "NOW" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.234"]);
        "NPER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.235"]);
        "NPV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.236"]);
        "OCT2BIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.237"]);
        "OCT2DEC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.238"]);
        "OCT2HEX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.239"]);
        "ODD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.240"]);
        "ODDFPRICE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.241"]);
        "ODDFYIELD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.242"]);
        "ODDLPRICE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.243"]);
        "ODDLYIELD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.244"]);
        // Reviewed safe for the reason in the comment above INDIRECT.
        "OFFSET" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.245"]);
        "OR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.246"]);
        "PEARSON" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.247"]);
        "PERCENTILE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.248"]);
        "PERCENTRANK" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.249"]);
        "PERMUT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.250"]);
        "PHONETIC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.251"]);
        "PI" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.252"]);
        "PMT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.253"]);
        "POISSON" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.254"]);
        "POWER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.255"]);
        "PPMT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.256"]);
        "PRICE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.257"]);
        "PRICEDISC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.258"]);
        "PRICEMAT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.259"]);
        "PROB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.260"]);
        "PRODUCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.261"]);
        "PROPER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.262"]);
        "PV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.263"]);
        "QUARTILE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.264"]);
        "QUOTIENT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.265"]);
        "RADIANS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.266"]);
        "RAND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.267"]);
        "RANDBETWEEN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.268"]);
        "RANK" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.269"]);
        "RATE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.270"]);
        "RECEIVED" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.271"]);
        "REPLACE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.272"]);
        "REPLACEB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.273"]);
        "REPT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.274"]);
        "RIGHT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.275"]);
        "RIGHTB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.276"]);
        "ROMAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.277"]);
        "ROUND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.278"]);
        "ROUNDDOWN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.279"]);
        "ROUNDUP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.280"]);
        "ROW" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.281"]);
        "ROWS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.282"]);
        "RSQ" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.283"]);
        "RTD" => safety: dangerous(Danger::ExternalData);
        "SEARCH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.285"]);
        "SEARCHB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.286"]);
        "SECOND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.287"]);
        "SERIESSUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.288"]);
        "SIGN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.289"]);
        "SIN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.290"]);
        "SINH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.291"]);
        "SKEW" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.292"]);
        "SLN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.293"]);
        "SLOPE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.294"]);
        "SMALL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.295"]);
        "SQRT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.296"]);
        "SQRTPI" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.297"]);
        "STANDARDIZE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.298"]);
        "STDEV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.299"]);
        "STDEVA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.300"]);
        "STDEVP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.301"]);
        "STDEVPA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.302"]);
        "STEYX" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.303"]);
        "SUBSTITUTE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.304"]);
        "SUBTOTAL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.305"]);
        "SUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.306"]);
        "SUMIF" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.307"]);
        "SUMIFS" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.308"]);
        "SUMPRODUCT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.309"]);
        "SUMSQ" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.310"]);
        "SUMX2MY2" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.311"]);
        "SUMX2PY2" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.312"]);
        "SUMXMY2" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.313"]);
        "SYD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.314"]);
        "T" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.315"]);
        "TAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.316"]);
        "TANH" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.317"]);
        "TBILLEQ" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.318"]);
        "TBILLPRICE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.319"]);
        "TBILLYIELD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.320"]);
        "TDIST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.321"]);
        "TEXT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.322"]);
        "TIME" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.323"]);
        "TIMEVALUE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.324"]);
        "TINV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.325"]);
        "TODAY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.326"]);
        "TRANSPOSE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.327"]);
        "TREND" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.328"]);
        "TRIM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.329"]);
        "TRIMMEAN" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.330"]);
        "TRUE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.331"]);
        "TRUNC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.332"]);
        "TTEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.333"]);
        "TYPE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.334"]);
        "UPPER" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.335"]);
        "USDOLLAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.336"]);
        "VALUE" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.337"]);
        "VAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.338"]);
        "VARA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.339"]);
        "VARP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.340"]);
        "VARPA" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.341"]);
        "VDB" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.342"]);
        "VLOOKUP" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.343"]);
        "WEEKDAY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.344"]);
        "WEEKNUM" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.345"]);
        "WEIBULL" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.346"]);
        "WORKDAY" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.347"]);
        "XIRR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.349"]);
        "XNPV" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.350"]);
        "YEAR" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.351"]);
        "YEARFRAC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.352"]);
        "YIELD" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.353"]);
        "YIELDDISC" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.354"]);
        "YIELDMAT" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.355"]);
        "ZTEST" => safety: reviewed(&["ECMA-376-1:2016 §18.17.7.356"]);
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

        // Unreviewed: no page at Microsoft that defines ANCHORARRAY was found;
        // the page on the spilled range operator
        // (https://support.microsoft.com/en-us/excel/spilled-range-operator)
        // describes `#` without naming ANCHORARRAY. That `J1#` is saved as
        // `_xlfn.ANCHORARRAY(J1)` is known from a workbook Excel for Mac saved
        // (the formula crate's tests of saved formulas), not from a definition.
        "ANCHORARRAY" => safety: unreviewed();
        "SINGLE" => safety: reviewed(&["https://support.microsoft.com/en-us/excel/implicit-intersection-operator"]);
        // https://support.microsoft.com/en-us/office/call-function-32d58445-e646-4ffd-8d5e-b45077a5e995
        "CALL" => safety: dangerous(Danger::ExternalCode);
        // https://support.microsoft.com/en-us/excel/functions/copilot-function
        "COPILOT" => safety: dangerous(Danger::Network);
        // https://support.microsoft.com/en-us/excel/functions/register-id-function
        "REGISTER.ID" => safety: dangerous(Danger::ExternalCode);
    }
}
