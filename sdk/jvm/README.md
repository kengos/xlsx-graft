# xlsx-graft (Java)

Java bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
shared C library (`libxlsx_graft_capi`) called through
[JNA](https://github.com/java-native-access/jna).

This is a skeleton: the only call so far is the engine's version.

```java
import io.github.kengos.xlsxgraft.Diagnostic;
import io.github.kengos.xlsxgraft.Engine;
import io.github.kengos.xlsxgraft.Result;

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
The one thing that throws is a missing engine library
(`io.github.kengos.xlsxgraft.LibraryNotFoundException`).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `Engine.version("/path/to/libxlsx_graft_capi.so")`
3. a library in `native/` on the classpath (no jar ships one yet)

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
