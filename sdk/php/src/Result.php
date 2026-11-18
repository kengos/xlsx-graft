<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * What every call into the engine returns: whether the call worked, the value
 * when it did, why it did not when it stopped, and the diagnostics either
 * way. `value` is null on a failure, except for a view ({@see Viewer}).
 *
 * `error` says why a read stopped short: the workbook or a sheet asked for
 * was refused, or the CLI failed. A read can fail with no `error` too: one
 * that a problem in its sheet stopped, whose diagnostics hold an error, as
 * `xlsx-graft read`'s `ok` is false then. The engine's version, which is not
 * a read, says why it failed in its diagnostics alone. A verify
 * ({@see Verifier}) has no value. Its `error` is set when a file could not
 * be read or written (`io`), a path or a rule holds a NUL (`invalid_arg`),
 * or the CLI gave no result (`exit_status`, `invalid_output`). The faults of
 * the workbook, the policy and the rules are in its diagnostics.
 *
 * A view ({@see Viewer}) is the one result that has a value when it fails: a
 * {@see Tree} or a {@see Grid}, holding what the engine could read (a workbook
 * that is broken gives a tree with the parts that were read, perhaps none; a
 * sheet that is not found gives a grid whose `sheet` is null). Its value is
 * null only when no result was got: `error` is `exit_status`,
 * `invalid_output`, `invalid_arg` (a path or an argument holds a NUL), or `io`
 * for a temporary file that cannot be written. A file that cannot be opened is
 * an `io` `error` with a value.
 *
 * @template-covariant T
 */
final readonly class Result
{
    /**
     * @param T|null $value
     * @param list<Diagnostic> $diagnostics
     */
    private function __construct(
        public bool $success,
        public mixed $value,
        public array $diagnostics,
        public ?Diagnostic $error = null,
    ) {}

    /**
     * @template V
     *
     * @param V $value
     * @param list<Diagnostic> $diagnostics
     *
     * @return self<V>
     */
    public static function ok(mixed $value, array $diagnostics = []): self
    {
        return new self(true, $value, $diagnostics);
    }

    /**
     * @template V = never
     *
     * @param list<Diagnostic> $diagnostics
     * @param V|null $value what the call could read before it failed; only a view ({@see Viewer}) gives one
     *
     * @return self<V>
     */
    public static function failure(array $diagnostics, ?Diagnostic $error = null, mixed $value = null): self
    {
        /** @var self<V> $failure a failure without a value is a result of every type */
        $failure = new self(false, $value, $diagnostics, $error);

        return $failure;
    }

    public function failed(): bool
    {
        return !$this->success;
    }
}
