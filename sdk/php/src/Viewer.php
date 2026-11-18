<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Shows the parts of a workbook and the cells of its sheets through the
 * engine's CLI: the engine's `tree` and `describe-sheet` views. The limits a
 * workbook is read under are the CLI's defaults, and so are the limits of what
 * is shown; a viewer takes no policy. The text form of the views is not
 * offered.
 *
 * A viewer does not change once it is made.
 *
 * ```php
 * $viewer = new Viewer();
 * $tree = $viewer->tree('book.xlsx')->value;
 * foreach ($tree?->parts ?? [] as $part) {
 *     echo "{$part->name} {$part->contentType}\n";
 * }
 *
 * $result = $viewer->describeSheet('book.xlsx', sheet: 'Data', range: 'A1:H50', maxRows: 20);
 * foreach ($result->value?->rows ?? [] as $row) {
 *     foreach ($row->cells as $cell) {
 *         echo "{$cell->ref} ", var_export($cell->value, true), "\n";
 *     }
 * }
 * ```
 *
 * The result of a view is a `Result<Tree>` or a `Result<Grid>`: it is
 * successful when the engine's `ok` is true, and its value is there also when
 * it is not. The value is null only when no result was got (`exit_status`,
 * `invalid_output`, `invalid_arg`, or `io` for a temporary file that cannot be
 * written); see {@see Result}.
 */
final class Viewer
{
    private readonly string $cli;

    /**
     * @param string|null $cli the CLI's path, used when `XLSX_GRAFT_CLI` is not set
     *
     * @throws CliNotFoundException when no CLI is found
     */
    public function __construct(?string $cli = null)
    {
        $this->cli = Cli::find($cli, Cli::environment());
    }

    /**
     * Shows the parts of the workbook at `$path` and the relationships between
     * them. A path holding a NUL is `invalid_arg`, and the CLI is not run. A
     * file that cannot be opened is an `io` `error`, with a tree that has no
     * parts.
     *
     * @return Result<Tree>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public function tree(string $path): Result
    {
        /** @var Result<Tree> */
        return ViewRun::path($this->cli, ViewRun::TREE, [], $path);
    }

    /**
     * Shows the parts of the workbook whose bytes `$data` holds, written to a
     * temporary file in a directory that only the current user may enter (on
     * Windows, the directory takes the access list of PHP's temporary
     * directory), both removed after the run; a file that cannot be written is
     * `io`, with no value.
     *
     * @return Result<Tree>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public function treeBytes(string $data): Result
    {
        /** @var Result<Tree> */
        return ViewRun::bytes($this->cli, ViewRun::TREE, [], $data, sys_get_temp_dir());
    }

    /**
     * Shows the cells of a sheet of the workbook at `$path`. Without `$sheet`,
     * the first sheet is shown, whatever its kind.
     *
     * A sheet that is not found is a `sheet_not_found` `error`, with a grid
     * whose `sheet` is null. A path, a sheet name or a range holding a NUL is
     * `invalid_arg`, and the CLI is not run. This package checks a value only
     * for a NUL (`invalid_arg`); the CLI judges the rest: a
     * value it refuses (a range that is no A1 range, `$maxRows` or
     * `$maxColumns` out of range, a negative `$sheet`) throws an
     * \InvalidArgumentException with the CLI's message, which names the CLI's
     * option (`--range`, `--max-rows`).
     *
     * @param string|int|null $sheet a sheet's name, or its index from 0 (passed as `--sheet-index`)
     * @param string|null $range an A1 range such as `A1:H50`
     * @param int|null $maxRows the most rows to show (1 to 10000; the CLI's 100 without it)
     * @param int|null $maxColumns the most columns to show (1 to 1000; the CLI's 30 without it)
     *
     * @return Result<Grid>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when the CLI refuses a value
     */
    public function describeSheet(string $path, string|int|null $sheet = null, ?string $range = null, ?int $maxRows = null, ?int $maxColumns = null): Result
    {
        /** @var Result<Grid> */
        return ViewRun::path($this->cli, ViewRun::DESCRIBE_SHEET, self::options($sheet, $range, $maxRows, $maxColumns), $path);
    }

    /**
     * Shows the cells of a sheet of the workbook whose bytes `$data` holds,
     * written to a temporary file as {@see self::treeBytes()} does.
     *
     * @param string|int|null $sheet as {@see self::describeSheet()}
     *
     * @return Result<Grid>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when the CLI refuses a value
     */
    public function describeSheetBytes(string $data, string|int|null $sheet = null, ?string $range = null, ?int $maxRows = null, ?int $maxColumns = null): Result
    {
        /** @var Result<Grid> */
        return ViewRun::bytes($this->cli, ViewRun::DESCRIBE_SHEET, self::options($sheet, $range, $maxRows, $maxColumns), $data, sys_get_temp_dir());
    }

    /**
     * The CLI's arguments for the values given, each `--name=value` in one
     * argument so a value that starts with `-` is not read as a flag.
     *
     * @return list<string>
     */
    private static function options(string|int|null $sheet, ?string $range, ?int $maxRows, ?int $maxColumns): array
    {
        $options = [];
        if ($sheet !== null) {
            $options[] = is_int($sheet) ? "--sheet-index={$sheet}" : "--sheet={$sheet}";
        }
        $range === null || $options[] = "--range={$range}";
        $maxRows === null || $options[] = "--max-rows={$maxRows}";
        $maxColumns === null || $options[] = "--max-columns={$maxColumns}";

        return $options;
    }
}
