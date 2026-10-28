//! The built-in formats: every id the standard lists, and its format code
//! read by the classifier to the same kind.

use super::super::format::classify;
use super::{FormatKind, builtin};

/// Every id §18.8.30 lists with a format code it reads to the same kind:
/// the code of all languages, Excel's (MS-OI29500 on §18.8.30, note a)
/// where it differs, and the Japanese one for 27–36 and 50–58.
const CODES: [(u32, &str); 47] = [
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "m/d/yyyy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yyyy h:mm"),
    (27, r#"[$-411]ge.m.d"#),
    (28, r#"[$-411]ggge"年"m"月"d"日""#),
    (29, r#"[$-411]ggge"年"m"月"d"日""#),
    (30, "m/d/yy"),
    (31, r#"yyyy"年"m"月"d"日""#),
    (32, r#"h"時"mm"分""#),
    (33, r#"h"時"mm"分"ss"秒""#),
    (34, r#"yyyy"年"m"月""#),
    (35, r#"m"月"d"日""#),
    (36, "[$-411]ge.m.d"),
    (37, "#,##0_);(#,##0)"),
    (38, "#,##0_);[Red](#,##0)"),
    (39, "#,##0.00_);(#,##0.00)"),
    (40, "#,##0.00_);[Red](#,##0.00)"),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mm:ss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
    (50, "[$-411]ge.m.d"),
    (51, r#"[$-411]ggge"年"m"月"d"日""#),
    (52, r#"yyyy"年"m"月""#),
    (53, r#"m"月"d"日""#),
    (54, r#"[$-411]ggge"年"m"月"d"日""#),
    (55, r#"yyyy"年"m"月""#),
    (56, r#"m"月"d"日""#),
    (57, "[$-411]ge.m.d"),
    (58, r#"[$-411]ggge"年"m"月"d"日""#),
];

#[test]
fn every_listed_id_has_the_kind_its_format_code_reads_to() {
    for (id, code) in CODES {
        let kind = builtin(id);
        assert!(kind.is_some(), "{id}");
        assert_eq!(kind, Some(classify(code)), "{id}: {code}");
    }
}

#[test]
fn the_thai_ids_have_the_kinds_of_their_codes() {
    use FormatKind::{Date, DateTime, Duration, Number, Time};
    // ว/ด/ปปปป is day, month, year; ช:นน hours and minutes; นน:ทท minutes
    // and seconds; [ช]:นน:ทท elapsed hours.
    let thai = [
        (59, Number),
        (60, Number),
        (61, Number),
        (62, Number),
        (67, Number),
        (68, Number),
        (69, Number),
        (70, Number),
        (71, Date),
        (72, Date),
        (73, Date),
        (74, Date),
        (75, Time),
        (76, Time),
        (77, DateTime),
        (78, Time),
        (79, Duration),
        (80, Time),
        (81, Date),
    ];
    for (id, kind) in thai {
        assert_eq!(builtin(id), Some(kind), "{id}");
    }
}

#[test]
fn an_id_the_standard_does_not_list_is_none() {
    let listed: Vec<u32> = (CODES.iter().map(|&(id, _)| id))
        .chain((59..=62).chain(67..=81))
        .collect();
    for id in 0..=200 {
        assert_eq!(builtin(id).is_some(), listed.contains(&id), "{id}");
    }
    assert_eq!(builtin(u32::MAX), None);
}
