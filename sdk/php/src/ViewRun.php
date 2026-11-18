<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One run of `xlsx-graft tree` or `xlsx-graft describe-sheet`: when the caller
 * gives bytes, the workbook's bytes in a file in a directory only the user that
 * runs PHP may enter; the arguments; and the result, with the file and its
 * directory removed after the run however it ends.
 *
 * @internal
 */
final class ViewRun
{
    /** The command that shows the parts of a workbook. */
    public const TREE = 'tree';

    /** The command that shows the cells of a sheet. */
    public const DESCRIBE_SHEET = 'describe-sheet';

    /**
     * Shows the workbook at `$path`.
     *
     * @param self::TREE|self::DESCRIBE_SHEET $view the CLI's command
     * @param list<string> $options the arguments of the caller's choices (`--range=A1:C3`)
     *
     * @return Result<Tree|Grid>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when the view is `describe-sheet` and the CLI refuses an option
     */
    public static function path(string $cli, string $view, array $options, string $path): Result
    {
        if (str_contains($path, "\0")) {
            return Result::failure([], new Diagnostic('invalid_arg', 'a path holds no NUL'));
        }

        return self::run($cli, $view, $options, '', $path, null);
    }

    /**
     * Shows the workbook whose bytes `$data` holds, written to a file of its
     * own in `$dir`.
     *
     * @param self::TREE|self::DESCRIBE_SHEET $view the CLI's command
     * @param list<string> $options the arguments of the caller's choices
     *
     * @return Result<Tree|Grid>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when the view is `describe-sheet` and the CLI refuses an option
     */
    public static function bytes(string $cli, string $view, array $options, string $data, string $dir): Result
    {
        return self::run($cli, $view, $options, $dir, '', $data);
    }

    /**
     * @param list<string> $options
     *
     * @return Result<Tree|Grid>
     */
    private static function run(string $cli, string $view, array $options, string $dir, string $path, ?string $data): Result
    {
        foreach ($options as $option) {
            if (str_contains($option, "\0")) {
                return Result::failure([], new Diagnostic('invalid_arg', 'an argument holds no NUL'));
            }
        }
        $file = null;
        try {
            if ($data !== null) {
                try {
                    $file = $path = TemporaryFile::write($dir, '.xlsx', $data);
                } catch (\RuntimeException $e) {
                    return Result::failure([], new Diagnostic('io', "the workbook could not be written to a temporary file ({$e->getMessage()})"));
                }
            }
            [$status, $signal, $stdout, $stderr] = CliProcess::run($cli, [$view, '--format', 'json', ...$options, '--', $path]);

            return $view === self::TREE
                ? ViewReport::read($status, $signal, $stdout, $stderr, false, ViewContent::tree(...), 'a boolean "ok" and the parts of the workbook')
                : ViewReport::read($status, $signal, $stdout, $stderr, true, ViewContent::grid(...), 'a boolean "ok" and the cells of the sheet');
        } finally {
            $file === null || TemporaryFile::remove($file);
        }
    }
}
