<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Verifies workbooks through the engine's CLI, under the host's policy (given
 * to the constructor) and the rules of the caller (given to each call).
 *
 * A verifier does not change once it is made. The policy is kept as JSON, so
 * changing the array it was made from afterwards changes nothing.
 *
 * ```php
 * $verifier = new Verifier(policy: ['max_formula_bytes' => 32768]);
 * $result = $verifier->verify('book.xlsx', targets: ['excel >= 2016'], allowParts: ['macro']);
 * foreach ($result->diagnostics as $d) {
 *     fwrite(STDERR, "{$d->severity} {$d->code}: {$d->message}\n");
 * }
 * ```
 */
final class Verifier
{
    private readonly string $cli;

    private readonly ?string $policy;

    /**
     * @param array<mixed>|null $policy the host's policy: the engine's limits, `fail_on`,
     *                                  `deny_scheduled` and `forbid` (`['functions' => [...], 'parts' => [...]]`).
     *                                  Keys are snake_case or camelCase, in the arrays inside it too;
     *                                  the values are not changed. null is the engine's defaults.
     *                                  It is not checked here: a policy the engine does not take is
     *                                  found by the first {@see self::verify()}, as `policy_invalid`
     *                                  diagnostics.
     * @param string|null $cli the CLI's path, used when `XLSX_GRAFT_CLI` is not set
     *
     * @throws CliNotFoundException when no CLI is found
     * @throws \InvalidArgumentException when the policy cannot be written as JSON
     */
    public function __construct(?array $policy = null, ?string $cli = null)
    {
        $this->cli = Cli::find($cli, Cli::environment());
        $this->policy = $policy === null ? null : VerifierPolicy::json($policy);
    }

    /**
     * Verifies the workbook at `$path`.
     *
     * The result is successful when the engine's `ok` is true; it has no
     * value. A workbook that cannot be read, or a temporary file that cannot be written, is an `io` `error`; a fault of the
     * rules is a `rules_invalid` or `target_invalid` diagnostic, and the
     * workbook is not opened. With `failOn: 'warn'` (or the policy's) the
     * result fails on a warning with no error diagnostic. A path or a rule
     * holding a NUL is `invalid_arg`, and the CLI is not run. The values are
     * not checked here: a `failOn` that is not `error` or `warn`, or a value
     * that is not UTF-8, is refused by the CLI as `exit_status`. A CLI that
     * gives no result is an `error` too: `exit_status` (it exited with a
     * status other than 0, 1 and 2, or a signal killed it) or `invalid_output`
     * (it exited with 0 and printed no result).
     *
     * @param array<string>|null $targets the versions the workbook must run in (`'excel >= 2016'`); a problem with one is an error
     * @param array<string>|null $bestEffort the versions it should run in; a problem with one is a warning
     * @param array<string>|null $allowFunctions functions refused by default to let through (without an `_xlfn.` prefix)
     * @param array<string>|null $allowParts kinds of part refused by default to let through (`macro`, `external_link`,
     *                                       `dde`, `ole`, `activex`, `data_connection`, `web_publish`)
     * @param string|null $failOn `'warn'` to fail on a warning too; the stricter of this and the policy's `fail_on` counts.
     *                            Only null leaves it out: `''` is passed to the CLI as `--fail-on=`, which refuses it (`exit_status`)
     *
     * @return Result<null>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when an array holds a value that is not a string
     */
    public function verify(
        string $path,
        ?array $targets = null,
        ?array $bestEffort = null,
        ?array $allowFunctions = null,
        ?array $allowParts = null,
        ?string $failOn = null,
    ): Result {
        $rules = self::rules($targets, $bestEffort, $allowFunctions, $allowParts, $failOn);

        return VerifyRun::path($this->cli, $this->policy, $path, $rules, sys_get_temp_dir());
    }

    /**
     * Verifies the workbook whose bytes `$data` holds, written to a temporary
     * file that only your user may read and removed after the run; a file
     * that cannot be written is `io`. Once the file is written, changes
     * to `$data` change nothing.
     *
     * @param array<string>|null $targets
     * @param array<string>|null $bestEffort
     * @param array<string>|null $allowFunctions
     * @param array<string>|null $allowParts
     *
     * @return Result<null> as {@see self::verify()}
     *
     * @throws CliNotFoundException when the CLI cannot be run
     * @throws \InvalidArgumentException when an array holds a value that is not a string
     */
    public function verifyBytes(
        string $data,
        ?array $targets = null,
        ?array $bestEffort = null,
        ?array $allowFunctions = null,
        ?array $allowParts = null,
        ?string $failOn = null,
    ): Result {
        $rules = self::rules($targets, $bestEffort, $allowFunctions, $allowParts, $failOn);

        return VerifyRun::bytes($this->cli, $this->policy, $data, $rules, sys_get_temp_dir());
    }

    /**
     * The CLI's arguments for the rules given, each `--name=value` in one
     * argument so a value that starts with `-` is not read as a flag.
     *
     * @param array<mixed>|null $targets
     * @param array<mixed>|null $bestEffort
     * @param array<mixed>|null $allowFunctions
     * @param array<mixed>|null $allowParts
     *
     * @return list<string>
     *
     * @throws \InvalidArgumentException when an array holds a value that is not a string
     */
    private static function rules(?array $targets, ?array $bestEffort, ?array $allowFunctions, ?array $allowParts, ?string $failOn): array
    {
        $arguments = [];
        foreach ([['targets', '--target', $targets], ['bestEffort', '--best-effort', $bestEffort], ['allowFunctions', '--allow-function', $allowFunctions], ['allowParts', '--allow-part', $allowParts]] as [$name, $flag, $values]) {
            foreach ($values ?? [] as $value) {
                is_string($value) || throw new \InvalidArgumentException("{$name} holds a value that is not a string");
                $arguments[] = "{$flag}={$value}";
            }
        }
        $failOn === null || $arguments[] = "--fail-on={$failOn}";

        return $arguments;
    }
}
