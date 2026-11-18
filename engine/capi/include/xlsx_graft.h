/*
 * xlsx-graft C ABI — the shared library the FFI SDKs load.
 *
 * Artifact: libxlsx_graft_capi.so / libxlsx_graft_capi.dylib /
 *           xlsx_graft_capi.dll
 *
 * This header is written by hand. `make engine:capi-abi` builds the library
 * and checks that the functions it exports are exactly the ones declared
 * here, and the crate's tests check that the codes below have the values
 * the library uses, and the structs the offsets given.
 *
 * THE RULES THIS SURFACE IS BUILT FROM
 *
 * 1. Nothing is NUL-terminated. A string crosses as a pointer plus a length,
 *    and the bytes are UTF-8. Do not hand a returned pointer to a function
 *    that expects a C string. An input of no bytes may come as NULL with a
 *    length of 0, as some FFIs pass an empty array.
 *
 * 2. What the library returns, the library owns. The version string is
 *    static: it lives as long as the library is loaded, and you never free
 *    it. Copy the bytes out if you need them past an unload. What a handle
 *    lends lives as its function says, never past the handle's close; the
 *    only thing you give back is the handle itself, to its close.
 *
 * 3. A failure is a status, never an unwind. Every entry point returns an
 *    int32_t status, and a panic inside the library is caught at the
 *    boundary and comes back as XLSX_GRAFT_ERR_PANIC. Read the
 *    out-parameters only on XLSX_GRAFT_OK.
 *
 * WHAT IT CANNOT CHECK
 *
 *    That a non-null pointer you pass points at a writable slot of the right
 *    type. A null pointer is refused with XLSX_GRAFT_ERR_NULL_ARG; anything
 *    else that is not a valid slot is undefined behaviour, as it would be
 *    with any C function. Nor can you check a returned length: the library
 *    guarantees it, so read exactly that many bytes.
 *
 *    Nor that a handle is still open. A handle is a pointer the library
 *    allocated, and its close frees it: passing it to any function after
 *    its close, closing it twice included, is undefined behaviour. A null
 *    handle is refused with XLSX_GRAFT_ERR_NULL_ARG, and closing a null
 *    handle does nothing and returns XLSX_GRAFT_OK, so a binding that sets
 *    its copy to NULL as it closes it can never use it closed. (A rows
 *    handle whose book was closed is not closed: see HANDLES.)
 *
 * HANDLES
 *
 *    A book (XlsxGraftBook) is a workbook opened to read its sheets' rows;
 *    a rows handle (XlsxGraftRows) reads one sheet's rows, one row a call.
 *    One book may have several rows handles open, read in turns.
 *
 *    Closing the book closes the rows of every rows handle opened from it,
 *    and the file they read: such a handle then answers
 *    xlsx_graft_rows_next with XLSX_GRAFT_ERR_CLOSED, and its own close is
 *    still to be called, to free what is left of it. So a binding need not
 *    track its rows handles to close the file: closing the book does. The
 *    other way round, a binding's rows object holds its book object, so
 *    that a garbage collector does not finalize the book, and with it the
 *    rows, while they are read.
 *
 *    The book keeps what went wrong, for xlsx_graft_book_diagnostics to
 *    say: the diagnostics, and the first failure (a sheet refused, the host
 *    failing while reading rows) as `xlsx-graft read` says it in its end
 *    line's `error`. Read them before closing the book. A sheet refused
 *    stays the book's error, even when another is read after: to look
 *    for a sheet, list them (xlsx_graft_book_sheets) rather than try to
 *    open one.
 *
 *    After XLSX_GRAFT_ERR_PANIC on a handle, its close included, the book
 *    and every rows handle of it answer every call but their closes with
 *    XLSX_GRAFT_ERR_PANIC, touching nothing: they share what the panic may
 *    have left half-changed. Close them; no diagnostics are left to read.
 *    A close that returns XLSX_GRAFT_ERR_PANIC has closed its handle all
 *    the same: never close it again.
 *
 * VERIFY
 *
 *    xlsx_graft_verify_path and xlsx_graft_verify_bytes verify a workbook
 *    and write a verify handle (XlsxGraftVerify) that holds the result;
 *    xlsx_graft_policy_check checks a policy alone and writes the same kind
 *    of handle. xlsx_graft_verify_result lends the result's JSON and
 *    xlsx_graft_verify_close frees the handle.
 *
 *    `options` is UTF-8 JSON, {"policy": {...}, "rules": {...}}, each key
 *    optional or null for the defaults; NULL with a length of 0, empty or
 *    only white space, or `null` means the default policy and the
 *    default rules. It may be 2 MiB at most.
 *    The policy is the host's limits and what it forbids, the rules the
 *    caller's (targets, bestEffort, functions.allow, parts.allow, failOn).
 *
 *    The result is {"ok": B, "failOn": "error" | "warn", "counts":
 *    {"error": N, "warn": N, "notice": N}, "error": null | {"code": "io",
 *    "message": M}, "diagnostics": [...]}. Its keys and types are fixed;
 *    a new key may be added, so ignore keys you do not know. XLSX_GRAFT_OK
 *    is returned whenever the engine tried to verify: a file that cannot be
 *    opened or read is the result's `error` (not XLSX_GRAFT_ERR_IO), and a
 *    fault of the policy or the rules is a diagnostic (policy_invalid,
 *    rules_invalid, target_invalid) with the workbook not opened.
 *    XLSX_GRAFT_ERR_INVALID_ARG is a fault of the code that built the call:
 *    a path that is not UTF-8 or `options` that is not such an object (not
 *    JSON, not an object, a key other than `policy` and `rules`, or larger
 *    than 2 MiB; a policy and rules that are each within 1 MiB can add up to
 *    more, so an SDK checks the size of the options it builds and reports a
 *    larger one as its caller's fault, not as its own bug).
 *
 *    None of these functions writes a message for
 *    xlsx_graft_error_message: one from an earlier call on the same thread
 *    stays as it was.
 *
 *    A verify handle does not change once it is made. Call, copy the result
 *    out, close, then parse, all in one function of yours: do not keep the
 *    handle as an object of yours. The JSON lives until the close, and
 *    xlsx_graft_verify_result returns the same bytes every time.
 *
 *    xlsx_graft_verify_bytes does not copy `data`: it reads it during the
 *    call only, so it must stay alive and unchanged until the call
 *    returns, and may be freed after.
 *
 *    After a call that returns XLSX_GRAFT_ERR_PANIC, no handle was written.
 *
 * VIEWS
 *
 *    xlsx_graft_tree_path and xlsx_graft_tree_bytes show the parts of a
 *    workbook and the relationships between them;
 *    xlsx_graft_describe_sheet_path and xlsx_graft_describe_sheet_bytes show
 *    the cells of one sheet. Each writes a view handle (XlsxGraftView) that
 *    holds the result; xlsx_graft_view_result lends its JSON and
 *    xlsx_graft_view_close frees the handle.
 *
 *    `options` is UTF-8 JSON, an object of at most 2 MiB; NULL with a
 *    length of 0, empty, only white space, or `null` is read as {} (every
 *    default). The options of a tree have no key. The options of
 *    describe_sheet are, each optional:
 *      "sheet": a sheet name (a string); not together with "sheetIndex",
 *      "sheetIndex": the sheet's index in the workbook's list (an integer
 *        from 0); without "sheet" and "sheetIndex" the first sheet is shown,
 *      "range": an A1 range such as "A1:H50": the cells shown start at its
 *        top left cell (at A1 without "range"), are at most "maxRows" rows
 *        and "maxColumns" columns, and never go past the range,
 *      "maxRows": 1 to 10000 (100 without it),
 *      "maxColumns": 1 to 1000 (30 without it).
 *    The limits a workbook is read under are the CLI's defaults; no policy
 *    is taken.
 *
 *    The result is {"ok": B, "error": null | {"code": C, "message": M},
 *    "diagnostics": [...], ...} and then the keys of what the view shows;
 *    docs/sdk.md (Listing parts, Describing a sheet) gives them. Its keys
 *    and types are fixed; a new key may be added, so ignore keys you do not
 *    know.
 *
 *    Which status a fault of the input returns. A call checks, in this
 *    order, the pointers, the path, the options as a whole, the values of
 *    the keys the view takes, and then the workbook, and returns the first
 *    fault it finds; *out_view is written only on XLSX_GRAFT_OK.
 *    - XLSX_GRAFT_ERR_INVALID_ARG: the path is not UTF-8, or the options are
 *      not a JSON object the engine reads (at most 2 MiB; empty, only white
 *      space or `null` is {}; a string with an unpaired surrogate escape
 *      such as "\ud800" and a number beyond a double's range such as 1e400
 *      are not JSON to the engine) whose keys are all keys of the view. It
 *      is a fault of the calling code (an SDK bug), and no message is
 *      written for xlsx_graft_error_message: one from an earlier call on
 *      the same thread stays as it was, so do not read it.
 *    - XLSX_GRAFT_ERR_REQUEST_INVALID (describe_sheet only): a fault of the
 *      value of a key the view takes. It is the caller's argument error:
 *      read xlsx_graft_error_message on the same thread right away; it
 *      gives the first fault, in English for a person.
 *    - XLSX_GRAFT_OK: otherwise. A file that cannot be opened or read (code
 *      `io`), a sheet that is not found (code `sheet_not_found`) and a
 *      workbook with errors are the result's `error` or `ok: false`, not a
 *      status; no message is written.
 *    The faults, in the order they are found (describe_sheet; a tree has
 *    rows 1 to 6 and 14 only, and any key is row 6):
 *       1. a pointer that must not be null is: NULL_ARG (a NULL path, data
 *          or options with a length of 0 is read as empty)
 *       2. the path is not UTF-8 (_path functions): INVALID_ARG
 *       3. the options are larger than 2 MiB: INVALID_ARG
 *       4. the options are not JSON the engine reads (not UTF-8, a syntax
 *          error, an unpaired surrogate escape, a number beyond a double's
 *          range): INVALID_ARG
 *       5. the options are neither an object nor `null` (`[]`, `1`, "x"):
 *          INVALID_ARG
 *       6. a key the view does not take, "path" included: INVALID_ARG
 *       7. "sheet" is not a string (`null` too): REQUEST_INVALID
 *       8. "sheetIndex" is not a JSON integer from 0 to the platform's
 *          size_t maximum (`null`, 5.0, -1, "1", a larger number):
 *          REQUEST_INVALID
 *       9. "sheet" and "sheetIndex" together: REQUEST_INVALID
 *      10. "range" is not a string: REQUEST_INVALID
 *      11. "maxRows" or "maxColumns" is not a JSON integer from 0 to
 *          2^64 - 1, in that order: REQUEST_INVALID
 *      12. "range" is not an A1 range: REQUEST_INVALID
 *      13. "maxRows" is outside 1 to 10000, then "maxColumns" outside 1 to
 *          1000: REQUEST_INVALID
 *      14. the workbook cannot be opened or read, the sheet is not found or
 *          cannot be chosen, or the workbook has errors: OK, in the result
 *    A key whose value is null is therefore not absent but row 7, 8, 10 or
 *    11. An SDK sends only the keys it was given, and does not call when
 *    its caller gave it a string that is not well-formed Unicode (for
 *    example in JS, String.prototype.isWellFormed() is false) or an integer
 *    beyond a double's range: it raises the language's own argument error.
 *
 *    A view handle does not change once it is made. Call, copy the result
 *    out, close, then parse, all in one function of yours: do not keep the
 *    handle as an object of yours. The JSON lives until the close, and
 *    xlsx_graft_view_result returns the same bytes every time.
 *
 *    xlsx_graft_tree_bytes and xlsx_graft_describe_sheet_bytes do not copy
 *    `data`: they read it during the call only, so it must stay alive and
 *    unchanged until the call returns, and may be freed after.
 *
 *    After a call that returns XLSX_GRAFT_ERR_PANIC, no handle was written.
 *
 * THREADING
 *
 *    The functions that take no handle (xlsx_graft_version,
 *    xlsx_graft_book_open_path, xlsx_graft_book_open_bytes,
 *    xlsx_graft_error_message, xlsx_graft_verify_path,
 *    xlsx_graft_verify_bytes, xlsx_graft_policy_check,
 *    xlsx_graft_tree_path, xlsx_graft_tree_bytes,
 *    xlsx_graft_describe_sheet_path, xlsx_graft_describe_sheet_bytes) may
 *    be called from several threads at once; verifications and views share
 *    no state.
 *
 *    A verify handle or a view handle, unlike a book, does not change after
 *    it is made, so xlsx_graft_verify_result or xlsx_graft_view_result may
 *    be called on it from any thread at once. Call xlsx_graft_verify_close
 *    or xlsx_graft_view_close once, and not while another call on the same
 *    handle is running.
 *
 *    A handle is not thread-safe. A book and every rows handle opened from
 *    it share state the library does not lock, so treat them as one: never
 *    call functions on two of them, or twice on one, from two threads at
 *    once. Using them from one thread after another is fine when your own
 *    lock, or the hand-over between the threads, orders the calls.
 *
 *    A close is a call like any other. A garbage collector's finalizer or
 *    cleaner thread that closes a handle while your program reads another
 *    handle of the same book races with it: close there only under the
 *    lock your program takes for that book, or not at all (report the
 *    handle as leaked instead).
 *
 *    xlsx_graft_error_message is per thread: it says why the last call on
 *    the calling thread failed with XLSX_GRAFT_ERR_IO or
 *    XLSX_GRAFT_ERR_REQUEST_INVALID.
 *
 *    A caught panic still runs Rust's process-wide panic hook, which by
 *    default prints a message to stderr.
 */

#ifndef XLSX_GRAFT_H
#define XLSX_GRAFT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- status codes ---------------------------------------------------- */

/* The call completed and wrote its out-parameters. */
#define XLSX_GRAFT_OK 0
/* A pointer argument that must not be null was null. Nothing was written. */
#define XLSX_GRAFT_ERR_NULL_ARG 1
/* A panic was caught at the boundary. The library is still usable; the call
 * is not, and its out-parameters may have been partly written, so read none
 * of them. */
#define XLSX_GRAFT_ERR_PANIC 2
/* The host failed, not the workbook: a file could not be opened, read or
 * written. xlsx_graft_error_message says why. */
#define XLSX_GRAFT_ERR_IO 3
/* A string argument is not UTF-8. */
#define XLSX_GRAFT_ERR_INVALID_ARG 4
/* The package or its workbook part cannot be read; the book's diagnostics
 * say why. */
#define XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE 5
/* No sheet has the name, or none is at the index. */
#define XLSX_GRAFT_ERR_SHEET_NOT_FOUND 6
/* More than one sheet has the name, letter case aside. */
#define XLSX_GRAFT_ERR_SHEET_AMBIGUOUS 7
/* The sheet is a chartsheet, a dialog sheet or a macro sheet, which have no
 * rows to read. */
#define XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET 8
/* The sheet's part is not known or cannot be read; the book's diagnostics
 * say why. */
#define XLSX_GRAFT_ERR_SHEET_UNREADABLE 9
/* The rows handle's book was closed, which closed its rows: the handle
 * reads no more, and only its close is left to call. */
#define XLSX_GRAFT_ERR_CLOSED 10
/* A value of a key of the options of a describe_sheet call is a fault (a
 * wrong type, a "range" that is not an A1 range, a "maxRows" that is 0,
 * "sheet" and "sheetIndex" together; see VIEWS).
 * xlsx_graft_error_message says why. *out_view is not written. */
#define XLSX_GRAFT_ERR_REQUEST_INVALID 11

/* ---- values ---------------------------------------------------------- */

/*
 * A cell's value. The kinds are the nine types of `xlsx-graft read
 * --format json`, and each carries what that type's JSON value says, so an
 * SDK reads a cell the same way from here as from the CLI.
 *
 * A date, a time or a date-time is the workbook's wall-clock time with no
 * time zone, counted in the workbook's date system and rounded to the
 * millisecond. An elapsed time is whole milliseconds, not wrapped at a day.
 */
#define XLSX_GRAFT_VALUE_EMPTY 0    /* no cell, or an empty one */
#define XLSX_GRAFT_VALUE_STRING 1   /* text, text_len */
#define XLSX_GRAFT_VALUE_NUMBER 2   /* number, always finite */
#define XLSX_GRAFT_VALUE_BOOLEAN 3  /* boolean: 0 or 1 */
#define XLSX_GRAFT_VALUE_DATE 4     /* year, month, day */
#define XLSX_GRAFT_VALUE_TIME 5     /* hour, minute, second, millisecond */
#define XLSX_GRAFT_VALUE_DATETIME 6 /* the fields of DATE and of TIME */
#define XLSX_GRAFT_VALUE_ELAPSED 7  /* elapsed, in milliseconds */
#define XLSX_GRAFT_VALUE_ERROR 8    /* text, text_len: as written, "#N/A" */

/*
 * The fields a value's kind does not name are zero, and text is NULL. text
 * is UTF-8 and NOT NUL-terminated. The layout has no padding of the
 * compiler's choosing; the offsets are those of a 64-bit target (on a
 * 32-bit one, text_len is at 36 and the struct is 40 bytes).
 */
typedef struct XlsxGraftValue {
    uint32_t kind;          /*  0: XLSX_GRAFT_VALUE_* */
    uint16_t year;          /*  4: 1900 to 9999 */
    uint8_t month;          /*  6: 1 to 12 */
    uint8_t day;            /*  7: 1 to 31 */
    uint8_t hour;           /*  8: 0 to 23 */
    uint8_t minute;         /*  9: 0 to 59 */
    uint8_t second;         /* 10: 0 to 59 */
    uint8_t boolean;        /* 11: 0 or 1 */
    uint16_t millisecond;   /* 12: 0 to 999 */
    uint16_t reserved;      /* 14: zero */
    double number;          /* 16 */
    uint64_t elapsed;       /* 24: milliseconds */
    const uint8_t *text;    /* 32 */
    size_t text_len;        /* 40; the struct is 48 bytes */
} XlsxGraftValue;

/* ---- sheets ---------------------------------------------------------- */

#define XLSX_GRAFT_SHEET_WORKSHEET 0   /* the one kind with rows to read */
#define XLSX_GRAFT_SHEET_CHARTSHEET 1  /* holds no cells */
#define XLSX_GRAFT_SHEET_DIALOGSHEET 2 /* holds no cells */
#define XLSX_GRAFT_SHEET_MACROSHEET 3  /* its cells are not read */
#define XLSX_GRAFT_SHEET_UNKNOWN 4     /* its part is not known; the
                                          diagnostics say why */

#define XLSX_GRAFT_STATE_VISIBLE 0
#define XLSX_GRAFT_STATE_HIDDEN 1      /* a user can show it again */
#define XLSX_GRAFT_STATE_VERY_HIDDEN 2 /* only a program can */

/* A sheet of a book. name is UTF-8 and NOT NUL-terminated. The offsets are
 * those of a 64-bit target (on a 32-bit one: 0, 4, 8, 12, and 16 bytes). */
typedef struct XlsxGraftSheet {
    const uint8_t *name;    /*  0 */
    size_t name_len;        /*  8 */
    uint32_t kind;          /* 16: XLSX_GRAFT_SHEET_* */
    uint32_t state;         /* 20: XLSX_GRAFT_STATE_*; the struct is 24 bytes */
} XlsxGraftSheet;

/* ---- handles --------------------------------------------------------- */

typedef struct XlsxGraftBook XlsxGraftBook;
typedef struct XlsxGraftRows XlsxGraftRows;
typedef struct XlsxGraftVerify XlsxGraftVerify;
typedef struct XlsxGraftView XlsxGraftView;

/* ---- functions ------------------------------------------------------- */

/*
 * The engine version, such as "0.0.0".
 *
 * On XLSX_GRAFT_OK, *out_ptr points at the version's UTF-8 bytes and
 * *out_len holds their count. The bytes are NOT NUL-terminated, are static,
 * and must not be freed. If either pointer is null the call returns
 * XLSX_GRAFT_ERR_NULL_ARG and writes nothing.
 */
int32_t xlsx_graft_version(const uint8_t **out_ptr, size_t *out_len);

/*
 * Why the last call on this thread that returned XLSX_GRAFT_ERR_IO or
 * XLSX_GRAFT_ERR_REQUEST_INVALID failed, in English for a person: *out_ptr
 * points at its UTF-8 bytes, *out_len holds their count (0 before any such
 * call). The bytes are NOT NUL-terminated, belong to the library, and live
 * until the next call on this thread that fails so, or the thread's end.
 */
int32_t xlsx_graft_error_message(const uint8_t **out_ptr, size_t *out_len);

/*
 * Opens the workbook at path (path_len bytes of UTF-8, NOT NUL-terminated)
 * and writes a book handle to *out_book; close it with
 * xlsx_graft_book_close. The file stays open until the book is closed,
 * which closes the rows of every rows handle opened from it too (see
 * HANDLES).
 *
 * XLSX_GRAFT_OK whenever the engine read the file, even when it found no
 * workbook in it: the book then refuses its sheets with
 * XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE and its diagnostics say why.
 * XLSX_GRAFT_ERR_IO when the file cannot be opened or read, or a temporary
 * file written; XLSX_GRAFT_ERR_INVALID_ARG when the path is not UTF-8 (read
 * such a file yourself and open its bytes).
 * Only XLSX_GRAFT_OK writes *out_book.
 */
int32_t xlsx_graft_book_open_path(const uint8_t *path, size_t path_len,
                                  XlsxGraftBook **out_book);

/*
 * As xlsx_graft_book_open_path, the workbook being the len bytes at data.
 * The bytes are copied before the call returns; free yours when you like.
 */
int32_t xlsx_graft_book_open_bytes(const uint8_t *data, size_t len,
                                   XlsxGraftBook **out_book);

/*
 * The book's sheets, in the order the workbook lists them, hidden ones
 * included: *out_sheets points at the first, *out_len holds their count.
 * The array and the names belong to the book and live until its close.
 * XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE when the workbook cannot be read.
 */
int32_t xlsx_graft_book_sheets(XlsxGraftBook *book,
                               const XlsxGraftSheet **out_sheets,
                               size_t *out_len);

/*
 * Every problem found so far, in the book and in the rows read from it, as
 * UTF-8 JSON (NOT NUL-terminated):
 * {"has_errors": B, "error": E, "diagnostics": [...]}, as `xlsx-graft read`
 * says them in its end line: has_errors whether a diagnostic is an error;
 * error null, or the first failure, {"code": C, "message": M}, C a code of
 * `read`'s `error` (workbook_unreadable, sheet_not_found, sheet_ambiguous,
 * sheet_not_worksheet, sheet_unreadable; workbook_unreadable from the
 * open on) or "io" (the host failed while reading rows); each diagnostic the object every host prints. A read that
 * a problem stopped has an error diagnostic, and no `error`. Call it after
 * reading the rows you want, for what they found. The bytes belong to the
 * book and live until the next call of this function on it, or its close.
 */
int32_t xlsx_graft_book_diagnostics(XlsxGraftBook *book,
                                    const uint8_t **out_ptr, size_t *out_len);

/*
 * Closes the book, and the rows of every rows handle opened from it, which
 * answer XLSX_GRAFT_ERR_CLOSED after: what they all lent is gone. NULL does
 * nothing. Always XLSX_GRAFT_OK (or XLSX_GRAFT_ERR_PANIC).
 */
int32_t xlsx_graft_book_close(XlsxGraftBook *book);

/*
 * Opens the rows of the sheet at index of the book's list (0 is the first,
 * hidden sheets and chartsheets counted) and writes a rows handle to
 * *out_rows; close it with xlsx_graft_rows_close. Refuses, writing nothing,
 * with XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE, XLSX_GRAFT_ERR_SHEET_NOT_FOUND,
 * XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET or XLSX_GRAFT_ERR_SHEET_UNREADABLE,
 * which the book keeps as its error, its message naming the sheet asked
 * for. xlsx_graft_book_sheets refusing an unreadable workbook keeps
 * workbook_unreadable so too.
 */
int32_t xlsx_graft_rows_open(XlsxGraftBook *book, size_t index,
                             XlsxGraftRows **out_rows);

/*
 * As xlsx_graft_rows_open, the sheet named name (name_len bytes of UTF-8,
 * NOT NUL-terminated), compared letter case aside. Refuses too with
 * XLSX_GRAFT_ERR_SHEET_AMBIGUOUS when more than one sheet has the name
 * (neither is chosen: readers differ on which it means), and
 * XLSX_GRAFT_ERR_INVALID_ARG when the name is not UTF-8.
 */
int32_t xlsx_graft_rows_open_name(XlsxGraftBook *book, const uint8_t *name,
                                  size_t name_len, XlsxGraftRows **out_rows);

/*
 * Reads the next row the sheet writes: *out_number is its number (from 1),
 * *out_values points at its values from column A to its last cell, and
 * *out_len holds their count. Only the rows the sheet writes come, in
 * order; a column with no cell between two that have one is
 * XLSX_GRAFT_VALUE_EMPTY. The values and their text belong to the rows
 * handle and live until the next call on it, or its close.
 *
 * At the end of the sheet, or when a problem in it stopped the reading
 * (the book's diagnostics say which), it writes 0, NULL and 0, and does
 * so on every call after. Tell the end by *out_number == 0, never by
 * *out_len: a row the sheet writes empty has its number and a length of
 * 0, and its *out_values is then not to be read. XLSX_GRAFT_ERR_IO when the host fails, writing
 * nothing, and the book keeps it as its error; no row comes after it
 * either. XLSX_GRAFT_ERR_CLOSED when the book was closed.
 */
int32_t xlsx_graft_rows_next(XlsxGraftRows *rows, uint32_t *out_number,
                             const XlsxGraftValue **out_values,
                             size_t *out_len);

/*
 * Closes the rows handle, its book closed or not. The values it lent are
 * gone. NULL does nothing. Always XLSX_GRAFT_OK (or XLSX_GRAFT_ERR_PANIC).
 */
int32_t xlsx_graft_rows_close(XlsxGraftRows *rows);

/*
 * Verifies the workbook at path (path_len bytes of UTF-8, NOT
 * NUL-terminated) under options (see VERIFY) and writes a verify handle to
 * *out_verify; close it with xlsx_graft_verify_close. XLSX_GRAFT_OK whenever
 * the engine tried: see VERIFY for what is the result's and what is a
 * status. XLSX_GRAFT_ERR_INVALID_ARG when the path is not UTF-8 (read such
 * a file yourself and verify its bytes) or options is not such an object (see VERIFY).
 * Only XLSX_GRAFT_OK writes *out_verify.
 */
int32_t xlsx_graft_verify_path(const uint8_t *path, size_t path_len,
                               const uint8_t *options, size_t options_len,
                               XlsxGraftVerify **out_verify);

/*
 * As xlsx_graft_verify_path, the workbook being the len bytes at data. The
 * bytes are NOT copied: they must stay alive and unchanged until the call
 * returns.
 */
int32_t xlsx_graft_verify_bytes(const uint8_t *data, size_t len,
                                const uint8_t *options, size_t options_len,
                                XlsxGraftVerify **out_verify);

/*
 * Checks the policy in the len bytes at policy: the policy's JSON object
 * itself, not wrapped in a request. Writes a verify handle whose result is
 * ok, or holds the policy_invalid diagnostics. XLSX_GRAFT_OK for any bytes,
 * UTF-8 and JSON or not (a fault of the policy is a diagnostic). An
 * empty policy (also NULL with a length of 0) or one of only white space is
 * the default policy; `null` is
 * not (it is not an object, so it is policy_invalid), unlike a `policy`
 * key that is null inside options. At most 1 MiB; larger is policy_invalid.
 */
int32_t xlsx_graft_policy_check(const uint8_t *policy, size_t len,
                                XlsxGraftVerify **out_verify);

/*
 * The result as UTF-8 JSON (NOT NUL-terminated, see VERIFY): *out_ptr
 * points at it, *out_len holds its length in bytes. The bytes belong to the handle
 * and live until its close; every call returns the same bytes.
 */
int32_t xlsx_graft_verify_result(const XlsxGraftVerify *verify,
                                 const uint8_t **out_ptr, size_t *out_len);

/*
 * Closes the verify handle; what it lent is gone. NULL does nothing.
 * Always XLSX_GRAFT_OK (or XLSX_GRAFT_ERR_PANIC).
 */
int32_t xlsx_graft_verify_close(XlsxGraftVerify *verify);

/*
 * Shows the parts of the workbook at path (path_len bytes of UTF-8, NOT
 * NUL-terminated) and their relationships, under options (see VIEWS: no
 * key), and writes a view handle to *out_view; close it with
 * xlsx_graft_view_close. Statuses: see VIEWS (a tree has faults 1 to 6 and
 * 14), and XLSX_GRAFT_ERR_PANIC as for every call.
 */
int32_t xlsx_graft_tree_path(const uint8_t *path, size_t path_len,
                             const uint8_t *options, size_t options_len,
                             XlsxGraftView **out_view);

/*
 * As xlsx_graft_tree_path, the workbook being the len bytes at data. The
 * bytes are NOT copied: they must stay alive and unchanged until the call
 * returns.
 */
int32_t xlsx_graft_tree_bytes(const uint8_t *data, size_t len,
                              const uint8_t *options, size_t options_len,
                              XlsxGraftView **out_view);

/*
 * Shows the cells of one sheet of the workbook at path (path_len bytes of
 * UTF-8, NOT NUL-terminated) under options (see VIEWS), and writes a view
 * handle to *out_view; close it with xlsx_graft_view_close. Statuses: see
 * VIEWS, and XLSX_GRAFT_ERR_PANIC as for every call.
 */
int32_t xlsx_graft_describe_sheet_path(const uint8_t *path, size_t path_len,
                                       const uint8_t *options,
                                       size_t options_len,
                                       XlsxGraftView **out_view);

/*
 * As xlsx_graft_describe_sheet_path, the workbook being the len bytes at
 * data. The bytes are NOT copied: they must stay alive and unchanged until
 * the call returns.
 */
int32_t xlsx_graft_describe_sheet_bytes(const uint8_t *data, size_t len,
                                        const uint8_t *options,
                                        size_t options_len,
                                        XlsxGraftView **out_view);

/*
 * The result as UTF-8 JSON (NOT NUL-terminated, see VIEWS): *out_ptr points
 * at it, *out_len holds its length in bytes. The bytes belong to the handle
 * and live until its close; every call returns the same bytes.
 * XLSX_GRAFT_OK; XLSX_GRAFT_ERR_NULL_ARG when view, out_ptr or out_len is
 * NULL (nothing is written); XLSX_GRAFT_ERR_PANIC.
 */
int32_t xlsx_graft_view_result(const XlsxGraftView *view,
                               const uint8_t **out_ptr, size_t *out_len);

/*
 * Closes the view handle; what it lent is gone. NULL does nothing.
 * XLSX_GRAFT_OK, or XLSX_GRAFT_ERR_PANIC (the handle is closed all the
 * same).
 */
int32_t xlsx_graft_view_close(XlsxGraftView *view);

#ifdef __cplusplus
}
#endif

#endif /* XLSX_GRAFT_H */
