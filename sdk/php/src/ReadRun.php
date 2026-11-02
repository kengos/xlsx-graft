<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One run of `xlsx-graft read --format json`: its rows a line at a time,
 * then what it came to, decided as docs/sdk.md § Reading rows says.
 *
 * @internal
 */
final class ReadRun
{
    /** The end line, once read. */
    private ?\stdClass $end = null;

    /** The line that broke the contract, or the last line read. */
    private string $last = '';

    private bool $invalid = false;

    private function __construct(
        private readonly CliProcess $process,
    ) {}

    /**
     * Starts `read` on the sheet `$which` names of the workbook at `$path`.
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public static function start(string $cli, string $path, string|int $which): self
    {
        $sheet = is_string($which) ? "--sheet={$which}" : "--sheet-index={$which}";

        return new self(CliProcess::start($cli, ['read', '--format', 'json', $sheet, '--', $path]));
    }

    /**
     * The next row; or why this package cannot give it (the run is then
     * to be stopped); or null once stdout ended, {@see self::outcome()}
     * then saying how.
     */
    public function next(Zone $zone): Row|Diagnostic|null
    {
        while (($line = $this->process->line()) !== null) {
            $this->last = $line;
            try {
                $this->end === null || throw new \UnexpectedValueException('a line after the end line');
                $object = CliJson::object($line);
                if (!property_exists($object, 'row')) {
                    CliJson::report($object);
                    CliJson::int($object, 'rows');
                    CliJson::field($object, 'sheet');
                    $this->end = $object;

                    continue;
                }
                $number = CliJson::int($object, 'row');
                $values = CellValues::call(CliJson::objects($object, 'cells'), $zone);

                return $values instanceof Diagnostic
                    ? new Diagnostic($values->code, "row {$number}: {$values->message}")
                    : new Row($number, $values);
            } catch (\UnexpectedValueException) {
                $this->invalid = true;
                // What the run comes to is read from its status once it ends:
                // a run that failed may have printed anything, so the rest
                // is read rather than the CLI stopped.
                $this->process->drain();
            }
        }

        return null;
    }

    /**
     * What the run came to, once {@see self::next()} returned null: the
     * end line's error and diagnostics, or why there is none.
     *
     * @return array{Diagnostic|null, list<array{string, Diagnostic}>}|Diagnostic
     */
    public function outcome(): array|Diagnostic
    {
        [$status, $signal, $stderr] = $this->process->finish();
        if (($status === 0 || $status === 1) && $signal === null && !$this->invalid && $this->end !== null) {
            return CliJson::report($this->end);
        }
        if ($status !== 0 || $signal !== null) {
            return CliDiagnostic::exitStatus($status, $signal, $stderr);
        }

        return CliDiagnostic::invalidOutput('the end line last', trim($this->last));
    }

    /**
     * Closes stdout, the caller having stopped taking rows, and waits for
     * the CLI to end: what it comes to is not a failure, whatever it is.
     */
    public function stop(): void
    {
        $this->process->finish();
    }
}
