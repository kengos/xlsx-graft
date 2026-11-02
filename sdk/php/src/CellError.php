<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * An error value a cell holds, such as `#N/A`. Never a plain string, which a
 * cell holding the text `#N/A` would be too.
 */
final readonly class CellError implements \Stringable
{
    /**
     * @param string $text the error value as written
     */
    public function __construct(
        public string $text,
    ) {}

    public function __toString(): string
    {
        return $this->text;
    }
}
