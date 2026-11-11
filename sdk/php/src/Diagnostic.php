<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One thing the engine reported about a call.
 *
 * The engine's own diagnostics (a read's {@see Result::$diagnostics}) carry
 * every field of the JSON every host prints, keys as written there, JSON
 * objects as arrays. One this package makes itself, such as a CLI run that
 * failed, is an `error` with `args` and `location` empty and no `decidedBy`
 * or `fix`.
 */
final readonly class Diagnostic
{
    /**
     * @param string $code the stable, machine-readable part
     * @param string $message for people
     * @param string $severity `error`, `warn` or `notice`
     * @param array<mixed> $args the values the message names, by name
     * @param array<mixed> $location where in the workbook: its part, cell or row, as the engine says
     * @param array<mixed>|null $decidedBy the rule or the source the engine followed, when it says
     * @param array<mixed>|null $fix how to resolve it, when the caller can
     */
    public function __construct(
        public string $code,
        public string $message,
        public string $severity = 'error',
        public array $args = [],
        public array $location = [],
        public ?array $decidedBy = null,
        public ?array $fix = null,
    ) {}
}
