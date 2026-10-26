<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * PHP bindings for xlsx-graft, an engine that reads, verifies, creates and
 * edits xlsx workbooks.
 *
 * The engine is its CLI, run as a subprocess; nothing here reimplements it.
 * Calls return a {@see Result} rather than throwing. What throws is a setup
 * with no runnable CLI in it ({@see CliNotFoundException}).
 *
 * ```php
 * $result = XlsxGraft::version();
 * if ($result->success) {
 *     echo $result->value;
 * }
 * ```
 */
final class XlsxGraft
{
    /**
     * The engine's version, such as `"0.0.0"`, on success.
     *
     * @param string|null $cli the CLI's path, used when `XLSX_GRAFT_CLI` is not set
     *
     * @return Result<string>
     *
     * @throws CliNotFoundException when no CLI is found or it cannot be run
     */
    public static function version(?string $cli = null): Result
    {
        return VersionCall::call(Cli::find($cli, Cli::environment()));
    }
}
