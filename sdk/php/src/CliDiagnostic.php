<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * The diagnostics of a CLI run that did not give a result
 * (docs/sdk.md § Running the CLI).
 *
 * The codes are provisional, as the other SDKs' status codes are: the engine's
 * own diagnostics replace them once the CLI reports some.
 *
 * @internal
 */
final class CliDiagnostic
{
    /** How much of the CLI's output a message quotes, in characters. */
    public const LIMIT = 200;

    /** The CLI exited with a status other than 0, or a signal killed it. */
    public static function exitStatus(int $status, ?int $signal, string $stderr): Diagnostic
    {
        $reason = $signal === null ? "the CLI exited with status {$status}" : "the CLI was killed by signal {$signal}";

        return new Diagnostic('exit_status', self::quoting($reason, $stderr));
    }

    /**
     * The CLI exited with 0, but printed no JSON object with `$expected`, what
     * the call reads from it (`a string "version"`).
     */
    public static function invalidOutput(string $expected, string $stdout): Diagnostic
    {
        return new Diagnostic('invalid_output', self::quoting("the CLI printed no JSON object with {$expected}", $stdout));
    }

    /**
     * `$reason`, followed by the start of `$output` when there is any.
     *
     * Control characters and line or paragraph separators become spaces, so
     * the quote stays on one line, and
     * output that is not UTF-8 keeps its printable ASCII alone. The cut is in
     * characters, never through one.
     */
    private static function quoting(string $reason, string $output): string
    {
        $text = trim(preg_replace('/[\p{C}\p{Zl}\p{Zp}]+/u', ' ', $output) ?? (string) preg_replace('/[^\x20-\x7e]+/', ' ', $output));
        if ($text === '') {
            return $reason;
        }

        $cut = preg_match('/^.{'.self::LIMIT.'}(?=.)/su', $text, $head) === 1 ? $head[0].'...' : $text;

        return "{$reason}: {$cut}";
    }
}
