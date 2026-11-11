<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * The state of one open book that {@see Book} and {@see Sheet} share: the
 * workbook's path, the runs of the CLI reading it, and what each run came
 * to.
 *
 * Every list of the sheets and every sheet's rows is a run of the CLI of
 * its own, which opens the workbook again. So each reports the problems of
 * opening it again; a diagnostic is kept as many times as one run gave it. PHP runs the
 * caller's code and this on one thread, and raises nothing between two of
 * its lines but running out of memory, so nothing here takes a lock.
 *
 * @internal
 */
final class BookCore
{
    private const CLOSED = "the book is closed: use it inside XlsxReader::open's callback";

    /** @var array<int, ReadRun> the runs open, by object id: each is stopped by its sheet's read or by the book's close */
    private array $runs = [];

    /** The first thing that stopped a call short. */
    private ?Diagnostic $error = null;

    /** @var list<Diagnostic> the engine's diagnostics */
    private array $diagnostics = [];

    /** @var array<string, int> how many times one run gave the diagnostic of each JSON, at most */
    private array $kept = [];

    /** @var list<SheetInfo>|null the sheets, once listed */
    private ?array $sheets = null;

    private bool $closed = false;

    /**
     * @param string|null $temporary a file of this book's own, removed when it closes
     */
    public function __construct(
        private readonly string $cli,
        private readonly string $path,
        private readonly Zone $zone,
        private readonly ?string $temporary = null,
    ) {}

    /**
     * The book of the workbook `$data` holds, written to a file of its own
     * in `$dir` (the CLI reads a path), which only this user may read; or
     * why it cannot be written.
     */
    public static function ofBytes(string $cli, string $data, Zone $zone, string $dir): self|Diagnostic
    {
        try {
            $path = TemporaryFile::write($dir, '.xlsx', $data);
        } catch (\RuntimeException $e) {
            return new Diagnostic('io', "the workbook could not be written to a temporary file ({$e->getMessage()})");
        }

        return new self($cli, $path, $zone, $path);
    }

    /** @throws ClosedException once the book is closed */
    public function check(): void
    {
        $this->closed && throw new ClosedException(self::CLOSED);
    }

    /**
     * The sheets, listed by one run of `xlsx-graft sheets`; none when it
     * failed, which the result says.
     *
     * @return list<SheetInfo>
     */
    public function sheets(): array
    {
        $this->check();

        return $this->sheets ??= $this->listSheets();
    }

    /**
     * The run reading the sheet `$which` names; null when this package
     * refuses the name, which the result says.
     */
    public function openRows(string|int $which): ?ReadRun
    {
        $this->check();
        if (is_string($which) && (!mb_check_encoding($which, 'UTF-8') || str_contains($which, "\0"))) {
            $this->fail(new Diagnostic('invalid_arg', 'a sheet name is UTF-8 text with no NUL'));

            return null;
        }
        $run = ReadRun::start($this->cli, $this->path, $which);
        $this->runs[spl_object_id($run)] = $run;

        return $run;
    }

    /**
     * The next row of `$run`; null at the end of the sheet, or when reading
     * it stopped short, which the result says.
     */
    public function nextRow(ReadRun $run): ?Row
    {
        $this->check();
        $next = $run->next($this->zone);
        if ($next instanceof Row) {
            return $next;
        }
        if ($next instanceof Diagnostic) {
            $this->fail($next);
            $this->closeRows($run);

            return null;
        }
        unset($this->runs[spl_object_id($run)]);
        $this->record($run->outcome());

        return null;
    }

    /** Stops `$run`, unless its end or the book's close did. */
    public function closeRows(ReadRun $run): void
    {
        if (isset($this->runs[spl_object_id($run)])) {
            unset($this->runs[spl_object_id($run)]);
            $run->stop();
        }
    }

    /** Stops every run still open and removes the book's own file. */
    public function close(): void
    {
        $this->closed = true;
        foreach ($this->runs as $run) {
            $run->stop();
        }
        $this->runs = [];
        if ($this->temporary !== null) {
            TemporaryFile::remove($this->temporary);
        }
    }

    /**
     * `$value` on success: no error, and no diagnostic an error.
     *
     * @template T
     *
     * @param T $value
     *
     * @return Result<T>
     */
    public function result(mixed $value): Result
    {
        $diagnostics = $this->diagnostics;
        $errors = array_filter($diagnostics, static fn(Diagnostic $d): bool => $d->severity === 'error');
        if ($this->error !== null || $errors !== []) {
            return Result::failure($diagnostics, $this->error);
        }

        return Result::ok($value, $diagnostics);
    }

    /** @return list<SheetInfo> */
    private function listSheets(): array
    {
        [$status, $signal, $stdout, $stderr] = CliProcess::run($this->cli, ['sheets', '--format', 'json', '--', $this->path]);
        if (($status === 0 || $status === 1) && $signal === null) {
            try {
                $listing = CliJson::object($stdout);
                $report = CliJson::report($listing);
                $sheets = array_map(self::sheet(...), CliJson::objects($listing, 'sheets'));
                $this->record($report);

                return $sheets;
            } catch (\UnexpectedValueException) {
                // Exit 1 with no object is the host failing, below.
            }
        }
        $this->fail($status !== 0 || $signal !== null
            ? CliDiagnostic::exitStatus($status, $signal, $stderr)
            : CliDiagnostic::invalidOutput('the sheets', trim($stdout)));

        return [];
    }

    private static function sheet(\stdClass $sheet): SheetInfo
    {
        return new SheetInfo(
            CliJson::int($sheet, 'index'),
            CliJson::string($sheet, 'name'),
            SheetKind::tryFrom(CliJson::string($sheet, 'kind')) ?? SheetKind::Unknown,
            SheetState::tryFrom(CliJson::string($sheet, 'state')) ?? SheetState::Unknown,
        );
    }

    /**
     * Keeps a run's error, unless one is kept, and its diagnostics but
     * those an earlier run gave as often.
     *
     * @param array{Diagnostic|null, list<array{string, Diagnostic}>}|Diagnostic $outcome a run's end, or why it has none
     */
    private function record(array|Diagnostic $outcome): void
    {
        if ($outcome instanceof Diagnostic) {
            $this->fail($outcome);

            return;
        }
        [$error, $diagnostics] = $outcome;
        if ($error !== null) {
            $this->fail($error);
        }
        $counted = [];
        foreach ($diagnostics as [$json, $diagnostic]) {
            $counted[$json] = ($counted[$json] ?? 0) + 1;
            if ($counted[$json] > ($this->kept[$json] ?? 0)) {
                $this->kept[$json] = $counted[$json];
                $this->diagnostics[] = $diagnostic;
            }
        }
    }

    /** The first failure is the one kept. */
    private function fail(Diagnostic $diagnostic): void
    {
        $this->error ??= $diagnostic;
    }
}
