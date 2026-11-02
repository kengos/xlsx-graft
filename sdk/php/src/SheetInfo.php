<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One sheet of a {@see Book}, as {@see Book::sheets()} lists it.
 */
final readonly class SheetInfo
{
    /**
     * @param int $index its place in the list, from 0, as {@see Book::sheet()} takes it
     */
    public function __construct(
        public int $index,
        public string $name,
        public SheetKind $kind,
        public SheetState $state,
    ) {}
}
