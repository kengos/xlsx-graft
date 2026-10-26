<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One thing the engine reported about a call.
 */
final readonly class Diagnostic
{
    /**
     * @param string $code the stable, machine-readable part
     * @param string $message for people
     */
    public function __construct(
        public string $code,
        public string $message,
    ) {}
}
