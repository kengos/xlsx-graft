/**
 * Java bindings for the xlsx-graft engine, which reads, verifies, creates and edits xlsx workbooks.
 *
 * <p>Every call returns a {@link io.github.kengos.xlsxgraft.Result} rather than throwing. The one
 * thing that throws is a setup that cannot reach the engine ({@link
 * io.github.kengos.xlsxgraft.LibraryNotFoundException}).
 */
package io.github.kengos.xlsxgraft;
