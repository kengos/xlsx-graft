<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One sheet of a {@see Book}, asked for by name or index.
 */
final class Sheet
{
    /** @internal {@see Book::sheet()} makes it. */
    public function __construct(
        private readonly BookCore $core,
        private readonly string|int $which,
    ) {}

    /**
     * Each row the sheet writes, in order, read one at a time as the
     * generator is, from a run of the CLI it starts at the first row asked
     * for: no more of the sheet is held than the row given and what the
     * pipe holds of the rows after it. Each call reads the sheet from its
     * start.
     *
     * Reading stops at the first problem in the sheet, the rows before it
     * having come, and gives nothing when the engine refuses the sheet;
     * either way the result fails, saying why. Stopping early (a `break`,
     * or dropping the generator) closes the CLI's stdout and waits for it
     * to stop, at the next output it flushes; that is no failure. A
     * generator neither read to its end nor dropped runs until the book's
     * callback returns.
     *
     * @return \Generator<int, Row, mixed, void>
     *
     * @throws ClosedException at the first row asked for after the book's callback returned
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public function rows(): \Generator
    {
        $run = $this->core->openRows($this->which);
        if ($run === null) {
            return;
        }
        try {
            while (($row = $this->core->nextRow($run)) !== null) {
                yield $row;
            }
        } finally {
            $this->core->closeRows($run);
        }
    }
}
