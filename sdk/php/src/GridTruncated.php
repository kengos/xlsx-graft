<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Which of a grid's limits cut what it shows.
 */
final readonly class GridTruncated
{
    /**
     * @param bool $rows whether rows were left out
     * @param bool $columns whether columns were left out
     * @param bool $merges whether merged ranges were left out
     */
    public function __construct(
        public bool $rows,
        public bool $columns,
        public bool $merges,
    ) {}
}
