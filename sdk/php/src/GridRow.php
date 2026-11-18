<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One row of a grid.
 */
final readonly class GridRow
{
    /**
     * @param int $row the row's number, from 1
     * @param list<GridCell> $cells the row's cells that hold a value, from the left
     */
    public function __construct(
        public int $row,
        public array $cells,
    ) {}
}
