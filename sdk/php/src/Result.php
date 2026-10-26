<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * What every call into the engine returns: whether the call worked, the value
 * when it did, and the diagnostics either way. `value` is null on a failure.
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
    public static function failure(array $diagnostics): self
    {
        /** @var self<never> $failure a failure holds no value, so it is a result of every type */
        $failure = new self(false, null, $diagnostics);

        return $failure;
    }

    public function failed(): bool
    {
        return !$this->success;
    }
}
