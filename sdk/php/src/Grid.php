<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * The cells of one sheet, the value of {@see Viewer::describeSheet()}. Names
 * and text are made for display (control characters removed, long text cut),
 * so they are not exact; read the rows with {@see XlsxReader} for exact
 * values.
 */
final readonly class Grid
{
    /**
     * @param SheetInfo|null $sheet the sheet shown, null when none was found
     * @param string|null $dimension the range the sheet declares, null when it declares none
     * @param string|null $used the range that holds cells, null when the sheet is empty
     * @param string|null $shown the range of the cells shown, null when none are
     * @param GridTruncated $truncated which limits cut what is shown
     * @param bool $complete whether the whole sheet was read
     * @param list<GridRow> $rows the rows shown
     * @param list<string> $merges the merged ranges (`A1:C1`)
     */
    public function __construct(
        public ?SheetInfo $sheet,
        public ?string $dimension,
        public ?string $used,
        public ?string $shown,
        public GridTruncated $truncated,
        public bool $complete,
        public array $rows,
        public array $merges,
    ) {}
}
