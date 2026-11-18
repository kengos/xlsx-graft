<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Where a relationship points, as the engine's `tree` shows it: a part of the
 * package, or a target outside it. One of `part` and `external` is null.
 */
final readonly class TreeTarget
{
    /**
     * @param string|null $part the part's name, null for an external target
     * @param string|null $external the external target, null for a part
     */
    public function __construct(
        public ?string $part,
        public ?string $external,
    ) {}
}
