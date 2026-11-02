/**
 * Java bindings for the xlsx-graft engine, which reads, verifies, creates and edits xlsx workbooks.
 *
 * <p>{@link io.github.kengos.xlsxgraft.XlsxReader} opens a workbook as a {@link
 * io.github.kengos.xlsxgraft.Book}, whose sheets' rows are read one at a time.
 *
 * <p>Every call returns a {@link io.github.kengos.xlsxgraft.Result} rather than throwing. What
 * throws is a setup that cannot reach the engine ({@link
 * io.github.kengos.xlsxgraft.LibraryNotFoundException}) and a call made wrongly, such as a book
 * used after it is closed.
 */
package io.github.kengos.xlsxgraft;
