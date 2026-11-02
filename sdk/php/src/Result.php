<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * What every call into the engine returns: whether the call worked, the value
 * when it did, why it did not when it stopped, and the diagnostics either
 * way. `value` is null on a failure.
 *
 * `error` says why a read stopped short: the workbook or a sheet asked for
 * was refused, or the CLI failed. A read can fail with no `error` too: one
 * that a problem in its sheet stopped, whose diagnostics hold an error, as
 * `xlsx-graft read`'s `ok` is false then. A call that is not a read (the
 * engine's version) says why it failed in its diagnostics alone.
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
     * @param list<Diagnostic> $diagnostics
     *
     * @return self<never>
     */
    public static function failure(array $diagnostics, ?Diagnostic $error = null): self
    {
        /** @var self<never> $failure a failure holds no value, so it is a result of every type */
        $failure = new self(false, null, $diagnostics, $error);

        return $failure;
    }

    public function failed(): bool
    {
        return !$this->success;
    }
}
