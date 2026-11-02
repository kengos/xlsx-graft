<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Opens a workbook to read its sheets' rows, through the engine's CLI.
 *
 * ```php
 * $result = XlsxReader::open('book.xlsx', function (Book $book): int {
 *     $count = 0;
 *     foreach ($book->sheet('Data')->rows() as $row) {
 *         echo $row->number, ': ', json_encode($row->values), "\n";
 *         $count++;
 *     }
 *
 *     return $count;
 * }, timezone: 'Asia/Tokyo');
 * foreach ($result->diagnostics as $d) {
 *     fwrite(STDERR, "{$d->severity} {$d->code}: {$d->message}\n");
 * }
 * ```
 */
final class XlsxReader
{
    /**
     * Opens the workbook at `$path`, hands it to `$read` as a {@see Book},
     * and closes it when `$read` returns, however it returns. An exception
     * `$read` throws is the caller's: it propagates, the book closed first.
     *
     * The result's value is what `$read` returned, on success. It fails
     * with `error` set when the workbook or a sheet asked for is refused, or
     * the CLI fails (a file that cannot be opened is one: `exit_status`,
     * quoting why); and without, when a problem stopped a sheet's rows, its
     * diagnostic an error. A path holding a NUL is `invalid_arg`, and
     * `$read` is not called.
     *
     * @template T
     *
     * @param callable(Book): T $read
     * @param string|\DateTimeZone $timezone the zone a date-time cell is read in: the workbook's are wall-clock times with no zone
     * @param string|null $cli the CLI's path, used when `XLSX_GRAFT_CLI` is not set
     *
     * @return Result<T>
     *
     * @throws CliNotFoundException when no CLI is found or it cannot be run
     * @throws \DateInvalidTimeZoneException for a zone PHP does not know
     */
    public static function open(string $path, callable $read, string|\DateTimeZone $timezone = 'UTC', ?string $cli = null): Result
    {
        $zone = self::zone($timezone);
        if (str_contains($path, "\0")) {
            return Result::failure([], new Diagnostic('invalid_arg', 'a path holds no NUL'));
        }

        return self::reading(new BookCore(Cli::find($cli, Cli::environment()), $path, $zone), $read);
    }

    /**
     * As {@see self::open()}, the workbook being the bytes of `$data`,
     * written to a temporary file of the book's own (the CLI reads a path),
     * which is removed when the book closes. A file that cannot be written
     * is `io`, and `$read` is not called.
     *
     * @template T
     *
     * @param callable(Book): T $read
     *
     * @return Result<T>
     *
     * @throws CliNotFoundException when no CLI is found or it cannot be run
     * @throws \DateInvalidTimeZoneException for a zone PHP does not know
     */
    public static function openBytes(string $data, callable $read, string|\DateTimeZone $timezone = 'UTC', ?string $cli = null): Result
    {
        $zone = self::zone($timezone);
        $cli = Cli::find($cli, Cli::environment());
        $core = BookCore::ofBytes($cli, $data, $zone, sys_get_temp_dir());

        return $core instanceof Diagnostic ? Result::failure([], $core) : self::reading($core, $read);
    }

    /**
     * @template T
     *
     * @param callable(Book): T $read
     *
     * @return Result<T>
     */
    private static function reading(BookCore $core, callable $read): Result
    {
        try {
            $value = $read(new Book($core));
        } finally {
            $core->close();
        }

        return $core->result($value);
    }

    private static function zone(string|\DateTimeZone $timezone): Zone
    {
        return new Zone($timezone instanceof \DateTimeZone ? $timezone : new \DateTimeZone($timezone));
    }
}
