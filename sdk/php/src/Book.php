<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * A workbook open to read, handed to the callback of
 * {@see XlsxReader::open()}: its sheets, and the rows of each through
 * {@see Book::sheet()}.
 *
 * Usable inside that callback only. When it returns or throws, the book is
 * closed, and with it every sheet's rows; using it then throws a
 * {@see ClosedException}.
 *
 * Nothing here throws for what the workbook holds. A workbook that cannot
 * be read lists no sheets, and a sheet the engine refuses (no such sheet,
 * not a worksheet) reads no rows; the {@see Result} fails saying why, as it
 * does when a problem stops a sheet's rows or the CLI fails.
 *
 * Each list of the sheets and each sheet's rows runs the CLI, which opens
 * the file again: a file changed in between is read as it is then.
 */
final class Book
{
    /** @internal {@see XlsxReader::open()} makes the book. */
    public function __construct(
        private readonly BookCore $core,
    ) {}

    /**
     * The sheets, in the order the workbook lists them, hidden ones and
     * every kind included; none when the workbook cannot be read.
     *
     * @return list<SheetInfo>
     *
     * @throws ClosedException after the callback returned
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public function sheets(): array
    {
        return $this->core->sheets();
    }

    /**
     * The sheet named `$which` (letter case aside) or at index `$which`
     * (from 0, hidden sheets and chartsheets counted); the first when not
     * given. Nothing is read until {@see Sheet::rows()}. A name or an index
     * no sheet has is the engine's to refuse there; to look for a sheet,
     * look in {@see Book::sheets()}: a refused sheet fails the result.
     *
     * @throws ClosedException after the callback returned
     * @throws \InvalidArgumentException for a negative index
     */
    public function sheet(string|int $which = 0): Sheet
    {
        $this->core->check();
        if (is_int($which) && $which < 0) {
            throw new \InvalidArgumentException("a sheet index is from 0, not {$which}");
        }

        return new Sheet($this->core, $which);
    }
}
