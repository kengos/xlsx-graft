//! Unit tests for the table's safety column: the checks every function's
//! safety must pass, the functions it classifies as dangerous, and the
//! helpers its rows are written with.

use super::{FUNCTIONS, SAFETY, dangerous, safe, unreviewed};
use crate::safety::{Danger, Dangerous, Review, Safe, Safety, Scheduled};
use std::collections::HashSet;

/// The headings of ECMA-376-1:2016 §18.17.7, Predefined Function
/// Definitions, in section order: `ECMA[n - 1]` is §18.17.7.n.
const ECMA: [&str; 356] = [
    "ABS",
    "ACCRINT",
    "ACCRINTM",
    "ACOS",
    "ACOSH",
    "ADDRESS",
    "AMORDEGRC",
    "AMORLINC",
    "AND",
    "AREAS",
    "ASC",
    "ASIN",
    "ASINH",
    "ATAN",
    "ATAN2",
    "ATANH",
    "AVEDEV",
    "AVERAGE",
    "AVERAGEA",
    "AVERAGEIF",
    "AVERAGEIFS",
    "BAHTTEXT",
    "BESSELI",
    "BESSELJ",
    "BESSELK",
    "BESSELY",
    "BETADIST",
    "BETAINV",
    "BIN2DEC",
    "BIN2HEX",
    "BIN2OCT",
    "BINOMDIST",
    "CEILING",
    "CELL",
    "CHAR",
    "CHIDIST",
    "CHIINV",
    "CHITEST",
    "CHOOSE",
    "CLEAN",
    "CODE",
    "COLUMN",
    "COLUMNS",
    "COMBIN",
    "COMPLEX",
    "CONCATENATE",
    "CONFIDENCE",
    "CONVERT",
    "CORREL",
    "COS",
    "COSH",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "COUNTIF",
    "COUNTIFS",
    "COUPDAYBS",
    "COUPDAYS",
    "COUPDAYSNC",
    "COUPNCD",
    "COUPNUM",
    "COUPPCD",
    "COVAR",
    "CRITBINOM",
    "CUBEKPIMEMBER",
    "CUBEMEMBER",
    "CUBEMEMBERPROPERTY",
    "CUBERANKEDMEMBER",
    "CUBESET",
    "CUBESETCOUNT",
    "CUBEVALUE",
    "CUMIPMT",
    "CUMPRINC",
    "DATE",
    "DATEDIF",
    "DATEVALUE",
    "DAVERAGE",
    "DAY",
    "DAYS360",
    "DB",
    "DCOUNT",
    "DCOUNTA",
    "DDB",
    "DEC2BIN",
    "DEC2HEX",
    "DEC2OCT",
    "DEGREES",
    "DELTA",
    "DEVSQ",
    "DGET",
    "DISC",
    "DMAX",
    "DMIN",
    "DOLLAR",
    "DOLLARDE",
    "DOLLARFR",
    "DPRODUCT",
    "DSTDEV",
    "DSTDEVP",
    "DSUM",
    "DURATION",
    "DVAR",
    "DVARP",
    "ECMA.CEILING",
    "EDATE",
    "EFFECT",
    "EOMONTH",
    "ERF",
    "ERFC",
    "ERROR.TYPE",
    "EVEN",
    "EXACT",
    "EXP",
    "EXPONDIST",
    "FACT",
    "FACTDOUBLE",
    "FALSE",
    "FDIST",
    "FIND",
    "FINDB",
    "FINV",
    "FISHER",
    "FISHERINV",
    "FIXED",
    "FLOOR",
    "FORECAST",
    "FREQUENCY",
    "FTEST",
    "FV",
    "FVSCHEDULE",
    "GAMMADIST",
    "GAMMAINV",
    "GAMMALN",
    "GCD",
    "GEOMEAN",
    "GESTEP",
    "GETPIVOTDATA",
    "GROWTH",
    "HARMEAN",
    "HEX2BIN",
    "HEX2DEC",
    "HEX2OCT",
    "HLOOKUP",
    "HOUR",
    "HYPERLINK",
    "HYPGEOMDIST",
    "IF",
    "IFERROR",
    "IMABS",
    "IMAGINARY",
    "IMARGUMENT",
    "IMCONJUGATE",
    "IMCOS",
    "IMDIV",
    "IMEXP",
    "IMLN",
    "IMLOG10",
    "IMLOG2",
    "IMPOWER",
    "IMPRODUCT",
    "IMREAL",
    "IMSIN",
    "IMSQRT",
    "IMSUB",
    "IMSUM",
    "INDEX",
    "INDIRECT",
    "INFO",
    "INT",
    "INTERCEPT",
    "INTRATE",
    "IPMT",
    "IRR",
    "ISBLANK",
    "ISERR",
    "ISERROR",
    "ISEVEN",
    "ISLOGICAL",
    "ISNA",
    "ISNONTEXT",
    "ISNUMBER",
    "ISO.CEILING",
    "ISODD",
    "ISPMT",
    "ISREF",
    "ISTEXT",
    "JIS",
    "KURT",
    "LARGE",
    "LCM",
    "LEFT",
    "LEFTB",
    "LEN",
    "LENB",
    "LINEST",
    "LN",
    "LOG",
    "LOG10",
    "LOGEST",
    "LOGINV",
    "LOGNORMDIST",
    "LOOKUP",
    "LOWER",
    "MATCH",
    "MAX",
    "MAXA",
    "MDETERM",
    "MDURATION",
    "MEDIAN",
    "MID",
    "MIDB",
    "MIN",
    "MINA",
    "MINUTE",
    "MINVERSE",
    "MIRR",
    "MMULT",
    "MOD",
    "MODE",
    "MONTH",
    "MROUND",
    "MULTINOMIAL",
    "N",
    "NA",
    "NEGBINOMDIST",
    "NETWORKDAYS",
    "NETWORKDAYS.INTL",
    "NOMINAL",
    "NORMDIST",
    "NORMINV",
    "NORMSDIST",
    "NORMSINV",
    "NOT",
    "NOW",
    "NPER",
    "NPV",
    "OCT2BIN",
    "OCT2DEC",
    "OCT2HEX",
    "ODD",
    "ODDFPRICE",
    "ODDFYIELD",
    "ODDLPRICE",
    "ODDLYIELD",
    "OFFSET",
    "OR",
    "PEARSON",
    "PERCENTILE",
    "PERCENTRANK",
    "PERMUT",
    "PHONETIC",
    "PI",
    "PMT",
    "POISSON",
    "POWER",
    "PPMT",
    "PRICE",
    "PRICEDISC",
    "PRICEMAT",
    "PROB",
    "PRODUCT",
    "PROPER",
    "PV",
    "QUARTILE",
    "QUOTIENT",
    "RADIANS",
    "RAND",
    "RANDBETWEEN",
    "RANK",
    "RATE",
    "RECEIVED",
    "REPLACE",
    "REPLACEB",
    "REPT",
    "RIGHT",
    "RIGHTB",
    "ROMAN",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "ROW",
    "ROWS",
    "RSQ",
    "RTD",
    "SEARCH",
    "SEARCHB",
    "SECOND",
    "SERIESSUM",
    "SIGN",
    "SIN",
    "SINH",
    "SKEW",
    "SLN",
    "SLOPE",
    "SMALL",
    "SQRT",
    "SQRTPI",
    "STANDARDIZE",
    "STDEV",
    "STDEVA",
    "STDEVP",
    "STDEVPA",
    "STEYX",
    "SUBSTITUTE",
    "SUBTOTAL",
    "SUM",
    "SUMIF",
    "SUMIFS",
    "SUMPRODUCT",
    "SUMSQ",
    "SUMX2MY2",
    "SUMX2PY2",
    "SUMXMY2",
    "SYD",
    "T",
    "TAN",
    "TANH",
    "TBILLEQ",
    "TBILLPRICE",
    "TBILLYIELD",
    "TDIST",
    "TEXT",
    "TIME",
    "TIMEVALUE",
    "TINV",
    "TODAY",
    "TRANSPOSE",
    "TREND",
    "TRIM",
    "TRIMMEAN",
    "TRUE",
    "TRUNC",
    "TTEST",
    "TYPE",
    "UPPER",
    "USDOLLAR",
    "VALUE",
    "VAR",
    "VARA",
    "VARP",
    "VARPA",
    "VDB",
    "VLOOKUP",
    "WEEKDAY",
    "WEEKNUM",
    "WEIBULL",
    "WORKDAY",
    "WORKDAY.INTL",
    "XIRR",
    "XNPV",
    "YEAR",
    "YEARFRAC",
    "YIELD",
    "YIELDDISC",
    "YIELDMAT",
    "ZTEST",
];

/// The version of xlsx-graft the tests are built from.
const CURRENT: [u64; 3] = [
    parse_u64(env!("CARGO_PKG_VERSION_MAJOR")),
    parse_u64(env!("CARGO_PKG_VERSION_MINOR")),
    parse_u64(env!("CARGO_PKG_VERSION_PATCH")),
];

/// The number `text` writes in decimal digits, in constant context.
const fn parse_u64(text: &str) -> u64 {
    let bytes = text.as_bytes();
    let mut number = 0;
    let mut at = 0;
    while at < bytes.len() {
        number = number * 10 + (bytes[at] - b'0') as u64;
        at += 1;
    }
    number
}

/// Whether `text` is a day of the calendar written as `YYYY-MM-DD`.
fn is_day(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |part: &str, len: usize| {
        (part.len() == len && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
    };
    match parts[..] {
        [year, month, day] => {
            let (Some(year), Some(month), Some(day)) =
                (number(year, 4), number(month, 2), number(day, 2))
            else {
                return false;
            };
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            let last = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap => 29,
                2 => 28,
                _ => return false,
            };
            (1..=last).contains(&day)
        }
        _ => false,
    }
}

/// Whether `text` names the Excel a review compared with, as
/// `Excel 365, YYYY-MM`.
fn is_excel_version(text: &str) -> bool {
    text.strip_prefix("Excel 365, ")
        .is_some_and(|month| is_day(&format!("{month}-01")))
}

/// The version of xlsx-graft `text` names as `MAJOR.MINOR.PATCH`, as
/// three numbers; `None` when `text` is not written so.
fn release(text: &str) -> Option<[u64; 3]> {
    let mut numbers = [0; 3];
    let mut parts = text.split('.');
    for number in &mut numbers {
        let part = parts.next()?;
        let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !digits || (part != "0" && part.starts_with('0')) {
            return None;
        }
        *number = part.parse().ok()?;
    }
    parts.next().is_none().then_some(numbers)
}

/// Whether `text` is the ID of a GitHub security advisory,
/// `GHSA-xxxx-xxxx-xxxx` in the letters GitHub uses.
fn is_advisory(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 4
        && groups[0] == "GHSA"
        && groups[1..].iter().all(|group| {
            group.len() == 4 && group.bytes().all(|b| b"23456789cfghjmpqrvwx".contains(&b))
        })
}

/// The section of ECMA-376-1:2016 §18.17.7 whose heading is `name`, as
/// a review cites it (`ECMA-376-1:2016 §18.17.7.306` for SUM); `None` for
/// a function ECMA-376 does not define.
fn section(name: &str) -> Option<String> {
    let index = ECMA.iter().position(|&heading| heading == name)?;
    Some(format!("ECMA-376-1:2016 §18.17.7.{}", index + 1))
}

/// Whether a review of the function `name` may cite `source`: its own
/// section of ECMA-376-1:2016 §18.17.7, or a page at Microsoft.
fn cites(name: &str, source: &str) -> bool {
    source.starts_with("https://support.microsoft.com/")
        || source.starts_with("https://learn.microsoft.com/")
        || section(name).is_some_and(|section| section == source)
}

/// What is wrong with the review of the function `name`, empty when
/// nothing is: a function ECMA-376 defines cites its section.
fn review_problems(name: &str, review: &Review) -> Vec<String> {
    let mut found = Vec::new();
    if !is_day(review.on) {
        found.push(format!("{name}: the review's day {:?}", review.on));
    }
    if !is_excel_version(review.version) {
        found.push(format!("{name}: the review's Excel {:?}", review.version));
    }
    if review.sources.is_empty() {
        found.push(format!("{name}: the review cites no source"));
    }
    for source in review.sources {
        if !cites(name, source) {
            found.push(format!("{name}: the review cites {source:?}"));
        }
    }
    if let Some(section) = section(name)
        && !review.sources.contains(&section.as_str())
    {
        found.push(format!("{name}: the review does not cite {section}"));
    }
    found
}

/// What is wrong with the safety of the function `name` in the version
/// `current` of xlsx-graft, empty when nothing is: `Scheduled::from` is
/// a version after `current`, and `Dangerous::denied_since` a version no
/// later than `current`.
fn problems(name: &str, safety: &Safety, current: [u64; 3]) -> Vec<String> {
    let mut found = Vec::new();
    match safety {
        Safety::ReviewedSafe(Safe { review, scheduled }) => {
            found.extend(review_problems(name, review));
            if let Some(scheduled) = scheduled
                && release(scheduled.from).is_none_or(|from| from <= current)
            {
                found.push(format!("{name}: denied from {:?}", scheduled.from));
            }
        }
        Safety::Dangerous(Dangerous {
            danger: _,
            denied_since,
            advisory,
        }) => {
            if let Some(since) = denied_since
                && release(since).is_none_or(|since| since > current)
            {
                found.push(format!("{name}: denied since {since:?}"));
            }
            if let Some(id) = advisory
                && !is_advisory(id)
            {
                found.push(format!("{name}: the advisory {id:?}"));
            }
            if advisory.is_some() && denied_since.is_none() {
                found.push(format!("{name}: an advisory with no version"));
            }
        }
        Safety::Unreviewed => {}
    }
    found
}

#[test]
fn every_safety_passes_the_checks() {
    let found: Vec<String> = SAFETY
        .iter()
        .flat_map(|row| problems(row.name, &row.safety, CURRENT))
        .collect();
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn every_function_has_one_safety() {
    let mut seen = HashSet::new();
    for row in SAFETY {
        let name = row.name;
        assert!(
            name.starts_with(|c: char| c.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.'),
            "{name}"
        );
        assert!(seen.insert(name), "{name} has two rows");
    }
    // The checked functions come first, in their order; then the functions
    // of ECMA-376 they leave out, in ECMA-376's order; then the functions
    // ECMA-376 does not define.
    let checked: Vec<_> = FUNCTIONS.iter().map(|row| row.name).collect();
    let mut expected = checked.clone();
    expected.extend(ECMA.iter().filter(|name| !checked.contains(name)));
    expected.extend(["ANCHORARRAY", "SINGLE", "CALL", "COPILOT", "REGISTER.ID"]);
    let names: Vec<_> = SAFETY.iter().map(|row| row.name).collect();
    assert_eq!(names, expected);
}

#[test]
fn every_function_ecma_376_defines_has_a_row() {
    let names: HashSet<_> = SAFETY.iter().map(|row| row.name).collect();
    let missing: Vec<_> = ECMA.iter().filter(|name| !names.contains(*name)).collect();
    assert!(missing.is_empty(), "{missing:?}");
    let unique: HashSet<_> = ECMA.iter().collect();
    assert_eq!(unique.len(), ECMA.len());
}

#[test]
fn only_the_listed_functions_are_dangerous_and_every_other_function_is_unreviewed() {
    let mut dangerous: Vec<(&str, Danger)> = SAFETY
        .iter()
        .filter_map(|row| match row.safety {
            Safety::Dangerous(Dangerous { danger, .. }) => Some((row.name, danger)),
            _ => None,
        })
        .collect();
    dangerous.sort_by_key(|&(name, _)| name);
    assert_eq!(
        dangerous,
        [
            ("CALL", Danger::ExternalCode),
            ("CELL", Danger::MachineInfo),
            ("COPILOT", Danger::Network),
            ("CUBEKPIMEMBER", Danger::ExternalData),
            ("CUBEMEMBER", Danger::ExternalData),
            ("CUBEMEMBERPROPERTY", Danger::ExternalData),
            ("CUBERANKEDMEMBER", Danger::ExternalData),
            ("CUBESET", Danger::ExternalData),
            ("CUBESETCOUNT", Danger::ExternalData),
            ("CUBEVALUE", Danger::ExternalData),
            ("DETECTLANGUAGE", Danger::Network),
            ("HYPERLINK", Danger::OpensLink),
            ("IMAGE", Danger::Network),
            ("INFO", Danger::MachineInfo),
            ("PY", Danger::Network),
            ("REGISTER.ID", Danger::ExternalCode),
            ("RTD", Danger::ExternalData),
            ("STOCKHISTORY", Danger::Network),
            ("TRANSLATE", Danger::Network),
            ("WEBSERVICE", Danger::Network),
        ]
    );
    // Every other function is not reviewed yet; none is reviewed safe.
    let unreviewed = SAFETY
        .iter()
        .filter(|row| row.safety == Safety::Unreviewed)
        .count();
    assert_eq!(unreviewed, SAFETY.len() - dangerous.len());
    // Every dangerous function has been denied from the first version.
    for row in SAFETY {
        if let Safety::Dangerous(found) = row.safety {
            assert_eq!(
                (found.denied_since, found.advisory),
                (None, None),
                "{}",
                row.name
            );
        }
    }
}

#[test]
fn the_checks_accept_each_field_in_its_format() {
    for day in [
        "2026-11-05",
        "2026-01-31",
        "2028-02-29",
        "2000-02-29",
        "2026-04-30",
    ] {
        assert!(is_day(day), "{day}");
    }
    assert!(is_excel_version("Excel 365, 2026-11"));
    assert_eq!(release("0.1.0"), Some([0, 1, 0]));
    assert_eq!(release("10.20.3"), Some([10, 20, 3]));
    assert!(is_advisory("GHSA-2c9h-7r4x-mq8f"));
    assert!(cites("SUM", "ECMA-376-1:2016 §18.17.7.306"));
    assert!(cites(
        "SUM",
        "https://support.microsoft.com/en-us/office/sum"
    ));
    assert!(cites(
        "XLOOKUP",
        "https://learn.microsoft.com/en-us/openspecs"
    ));
    let review = Review {
        on: "2026-11-05",
        version: "Excel 365, 2026-11",
        sources: &["ECMA-376-1:2016 §18.17.7.306"],
    };
    let current = [0, 2, 0];
    assert!(problems("SUM", &safe(review), current).is_empty());
    let scheduled = Safety::ReviewedSafe(Safe {
        review,
        scheduled: Some(Scheduled {
            from: "0.3.0",
            danger: Danger::ExternalData,
        }),
    });
    assert!(problems("SUM", &scheduled, current).is_empty());
    let closed = Safety::Dangerous(Dangerous {
        danger: Danger::Network,
        denied_since: Some("0.2.0"),
        advisory: Some("GHSA-2c9h-7r4x-mq8f"),
    });
    assert!(problems("SUM", &closed, current).is_empty());
    assert!(problems("SUM", &unreviewed(), current).is_empty());
    // A function ECMA-376 does not define cites a page at Microsoft only.
    let page = safe(Review {
        on: "2026-11-05",
        version: "Excel 365, 2026-11",
        sources: &["https://support.microsoft.com/en-us/office/xlookup"],
    });
    assert!(problems("XLOOKUP", &page, current).is_empty());
}

#[test]
fn the_checks_report_each_field_outside_its_format() {
    for day in [
        "2026-1-05",
        "2026-13-05",
        "2026-11-00",
        "26-11-05",
        "2026-11",
        "2026-1a-05",
        "2026-02-31",
        "2026-02-29",
        "2100-02-29",
        "2026-04-31",
        "2026-11-05-01",
    ] {
        assert!(!is_day(day), "{day}");
    }
    assert!(!is_excel_version("Excel 2024, 2026-11"));
    assert!(!is_excel_version("Excel 365, 2026-13"));
    for text in ["0.1", "0.1.0.0", "0.01.0", "0..0", "v0.1.0", "0.1.x"] {
        assert_eq!(release(text), None, "{text}");
    }
    for id in [
        "GHSA-2c9h-7r4x",
        "CVE-2c9h-7r4x-mq8f",
        "GHSA-2c9h-7r4x-mq8a",
        "GHSA-2c9-7r4x-mq8f",
    ] {
        assert!(!is_advisory(id), "{id}");
    }
    // Another function's section, a section past the last one, another
    // edition, and a page outside Microsoft.
    for source in [
        "ECMA-376-1:2016 §18.17.7.1",
        "ECMA-376-1:2016 §18.17.7.357",
        "ECMA-376-1:2012 §18.17.7.306",
        "https://example.com/sum",
        "http://support.microsoft.com/en-us/office/sum",
    ] {
        assert!(!cites("SUM", source), "{source}");
    }
    assert!(!cites("XLOOKUP", "ECMA-376-1:2016 §18.17.7.306"));
    let current = [0, 2, 0];
    let wrong = Safety::ReviewedSafe(Safe {
        review: Review {
            on: "2026-11",
            version: "Excel 365",
            sources: &["https://support.microsoft.com/en-us/office/sum"],
        },
        scheduled: Some(Scheduled {
            from: "next",
            danger: Danger::Network,
        }),
    });
    assert_eq!(
        problems("SUM", &wrong, current),
        [
            "SUM: the review's day \"2026-11\"",
            "SUM: the review's Excel \"Excel 365\"",
            "SUM: the review does not cite ECMA-376-1:2016 §18.17.7.306",
            "SUM: denied from \"next\"",
        ]
    );
    let sourceless = safe(Review {
        on: "2026-11-05",
        version: "Excel 365, 2026-11",
        sources: &[],
    });
    assert_eq!(
        problems("XLOOKUP", &sourceless, current),
        ["XLOOKUP: the review cites no source"]
    );
    // A Scheduled::from equal to the current version is not after it.
    let due = Safety::ReviewedSafe(Safe {
        review: Review {
            on: "2026-11-05",
            version: "Excel 365, 2026-11",
            sources: &["ECMA-376-1:2016 §18.17.7.1"],
        },
        scheduled: Some(Scheduled {
            from: "0.2.0",
            danger: Danger::Network,
        }),
    });
    assert_eq!(
        problems("SUM", &due, current),
        [
            "SUM: the review cites \"ECMA-376-1:2016 §18.17.7.1\"",
            "SUM: the review does not cite ECMA-376-1:2016 §18.17.7.306",
            "SUM: denied from \"0.2.0\"",
        ]
    );
    let closed = Safety::Dangerous(Dangerous {
        danger: Danger::Network,
        denied_since: Some("soon"),
        advisory: Some("GHSA-0000-0000-0000"),
    });
    assert_eq!(
        problems("WEBSERVICE", &closed, current),
        [
            "WEBSERVICE: denied since \"soon\"",
            "WEBSERVICE: the advisory \"GHSA-0000-0000-0000\"",
        ]
    );
    // A denied_since after the current version is later than it.
    let early = Safety::Dangerous(Dangerous {
        danger: Danger::Network,
        denied_since: Some("0.3.0"),
        advisory: None,
    });
    assert_eq!(
        problems("WEBSERVICE", &early, current),
        ["WEBSERVICE: denied since \"0.3.0\""]
    );
    let unannounced = Safety::Dangerous(Dangerous {
        danger: Danger::Network,
        denied_since: None,
        advisory: Some("GHSA-2c9h-7r4x-mq8f"),
    });
    assert_eq!(
        problems("WEBSERVICE", &unannounced, current),
        ["WEBSERVICE: an advisory with no version"]
    );
}

#[test]
fn the_helpers_build_the_safety_they_name() {
    assert_eq!(unreviewed(), Safety::Unreviewed);
    assert_eq!(
        dangerous(Danger::OpensLink),
        Safety::Dangerous(Dangerous {
            danger: Danger::OpensLink,
            denied_since: None,
            advisory: None,
        })
    );
    let review = Review {
        on: "2026-11-05",
        version: "Excel 365, 2026-11",
        sources: &["ECMA-376-1:2016 §18.17.7.306"],
    };
    assert_eq!(
        safe(review),
        Safety::ReviewedSafe(Safe {
            review,
            scheduled: None,
        })
    );
}
