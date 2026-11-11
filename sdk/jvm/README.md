# xlsx-graft (Java)

Java bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
shared C library (`libxlsx_graft_capi`) called through
[JNA](https://github.com/java-native-access/jna).

## Reading a sheet's rows

```java
import io.github.kengos.xlsxgraft.Book;
import io.github.kengos.xlsxgraft.Diagnostic;
import io.github.kengos.xlsxgraft.Result;
import io.github.kengos.xlsxgraft.Row;
import io.github.kengos.xlsxgraft.SheetInfo;
import io.github.kengos.xlsxgraft.XlsxReader;
import java.nio.file.Paths;

Book book = XlsxReader.open(Paths.get("book.xlsx"));
try (Book open = book) {
  for (SheetInfo sheet : open.sheets()) {
    System.out.println(sheet.index() + " " + sheet.name() + " (" + sheet.kind() + ", " + sheet.state() + ")");
  }

  for (Row row : open.sheet("Data").rows()) {
    System.out.println(row.number() + ": " + row.values());
  }
}

Result<Void> result = book.result();
if (result.error() != null) {
  System.err.println(result.error().code() + ": " + result.error().message());
}
for (Diagnostic d : result.diagnostics()) {
  System.err.println(d.severity() + " " + d.code() + ": " + d.message());
}
```

- `XlsxReader.openBytes(data)` reads a workbook from its bytes (copied
  when called).
- `book.sheet(...)` takes a name (letter case aside) or an index (from 0,
  hidden sheets and chartsheets counted); `book.sheet()` is the first
  sheet. To look for a sheet, look in `book.sheets()`: a sheet asked for
  and refused fails the result.
- `rows()` opens the sheet when called (a sheet refused fails the result
  then, iterated or not) and reads one row at a time as it is iterated,
  once. Only the rows
  the sheet writes come, each with its number from 1 and its values from
  column A, `null` where a column has none. Each call of `rows()` reads the
  sheet from its start.
- What the engine keeps of a sheet is freed when its last row is read, when
  its `Rows` is closed, or when the book is. To stop early and free it
  sooner, read the rows in a try-with-resources block of their own:
  `try (Rows rows = open.sheet("Data").rows()) { ... }`.
- The book is opened when `open` returns and closed when its
  try-with-resources block ends, however it ends, and its rows with it.
  Using it after throws `IllegalStateException`. An exception your block
  throws is yours: it comes out of the block, the book closed first.
- `book.result()` says how the reading went, once the book is closed. It
  has no value: to keep the rows read before a problem stopped the
  reading, collect them in a variable outside the block.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your block
  early: a file that cannot be opened or an unreadable workbook lists no
  sheets, a sheet refused reads no rows, and `result.error().code()` says
  why (`io`, `workbook_unreadable`, `sheet_not_found`, `sheet_ambiguous`,
  `sheet_not_worksheet`, `sheet_unreadable`, ...). A problem inside the
  sheet stops its rows; its diagnostic says what and where.
- One book may be read from several threads: every call takes the book's
  lock.

| Cell | Java |
| --- | --- |
| empty | `null` |
| string | `String` |
| number | `Double` |
| boolean | `Boolean` |
| date | `java.time.LocalDate` |
| time of day | `java.time.LocalTime` |
| date-time | `java.time.LocalDateTime` |
| elapsed time | `java.time.Duration` |
| error | `io.github.kengos.xlsxgraft.CellError` (`text()`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone, and are
given as such: no time zone is applied.

## Verifying a workbook

```java
import io.github.kengos.xlsxgraft.Diagnostic;
import io.github.kengos.xlsxgraft.Result;
import io.github.kengos.xlsxgraft.Verifier;
import io.github.kengos.xlsxgraft.VerifyRules;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;

Verifier verifier =
    new Verifier(Collections.singletonMap("maxFormulaBytes", 32768));
Result<Void> result =
    verifier.verify(
        Paths.get("book.xlsx"),
        VerifyRules.none()
            .targets(Arrays.asList("excel >= 2016"))
            .allowParts(Arrays.asList("macro")));
System.out.println(result.success() ? "ok" : "not ok");
for (Diagnostic d : result.diagnostics()) {
  System.err.println(d.severity() + " " + d.code() + ": " + d.message());
}
if (result.error() != null) {
  System.err.println(result.error().code() + ": " + result.error().message());
}
```

- `new Verifier(policy)` takes the host's limits and rules as a `Map<String, ?>`
  (the same policy file as the CLI's `--policy` works when a JSON library reads
  it into a Map with whole numbers as `Integer` or `Long`; one that reads them
  as `Double` writes `1000000.0`, which the engine may refuse as a limit). The
  keys are the engine's, in camelCase (`maxEntries`, `failOn`, `forbid`), and
  are not changed. A value is a Map, a List, a String, a Boolean, null or a
  number. The engine checks the policy there: one it does not take throws
  `PolicyInvalidException`, whose `diagnostics()` say what is wrong (for each
  key, a `policy_invalid` diagnostic).
- `verify(path)` and `verifyBytes(data)` verify under the policy alone; the
  overloads with a `VerifyRules` add the rules of the call, each optional:
  `targets`, `bestEffort`, `allowFunctions`, `allowParts` (lists of Strings)
  and `failOn` (`"warn"` to fail on a warning too).
  `VerifyRules` does not change: each method returns a new one.
- The result is successful when the engine's `ok` is true: no error
  diagnostic, and with `failOn("warn")` (the call's or the policy's) no
  warning either, so such a result can fail with no error diagnostic.
  `result.value()` is always `null`; `result.diagnostics()` has what was found.
  `result.error()` is set when the file cannot be read (`io`) or the call
  itself failed (`panic`, `null_arg`, `unknown_status`,
  `diagnostics_unreadable`). A fault of the rules is a `rules_invalid` or
  `target_invalid` diagnostic, and the workbook is not opened.
- A policy and rules that make more than 2 MiB of JSON, a value JSON cannot
  write or text that cannot be encoded as UTF-8 (an unpaired surrogate) throw
  `IllegalArgumentException`; a null path, data or rules throw
  `NullPointerException`. For a path holding an unpaired surrogate (only a
  Windows file name can) the package reads the file itself: if it cannot, the
  result is the `io` error and the rules are not checked.
- `verifyBytes` does not copy `data`: the call reads the array while it runs
  (JNA passes it through JNI, which may copy or pin it), so do not change the
  array until the call returns.
- A `Verifier` does not change once it is made: share one between threads, and
  call it from them at once.

## The engine's version

```java
import io.github.kengos.xlsxgraft.Engine;

Result<String> result = Engine.version();
if (result.success()) {
  System.out.println(result.value());
} else {
  for (Diagnostic d : result.diagnostics()) {
    System.err.println(d.code() + ": " + d.message());
  }
}
```

Calls return an `io.github.kengos.xlsxgraft.Result<T>` rather than throwing.
What throws is a missing engine library
(`io.github.kengos.xlsxgraft.LibraryNotFoundException`), a book used after it
is closed (`IllegalStateException`), and a call made wrongly
(`NullPointerException`, `IllegalArgumentException` for an index below 0,
`IllegalStateException` for a result read before the book is closed or rows
iterated twice).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `Engine.version("/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.open(path, "/path/to/libxlsx_graft_capi.so")`
3. the library in `native/<os>-<arch>/` on the classpath, such as
   `native/linux-x86-64/` (the directory JNA names for the platform). The
   `xlsx-graft` jar carries none: add the jar of the platform you run on, with
   its classifier (`linux-x86-64`, `linux-aarch64`, `darwin-aarch64`,
   `darwin-x86-64`, `win32-x86-64`)

The libraries are built for glibc Linux, macOS and Windows x64; on Alpine (musl)
the library is found and fails to load, and the error says so.

Java 8 or newer. One dependency, JNA. On Java 24 and later, JNA's loading of
a native library prints a warning unless the application runs with
`--enable-native-access=ALL-UNNAMED`. The Maven coordinates are
`io.github.kengos:xlsx-graft`; nothing is published yet.

## Development

Everything runs in Docker through `make` at the repository root; you need no
JDK or Gradle installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:jvm
```

`make sdk:format:jvm` applies the formatter the gate checks.
