<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Book;
use XlsxGraft\BookCore;
use XlsxGraft\CalendarDate;
use XlsxGraft\CellError;
use XlsxGraft\ClosedException;
use XlsxGraft\Diagnostic;
use XlsxGraft\Result;
use XlsxGraft\Row;
use XlsxGraft\SheetInfo;
use XlsxGraft\SheetKind;
use XlsxGraft\SheetState;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Tests\Support\Workbook;
use XlsxGraft\TimeOfDay;
use XlsxGraft\XlsxReader;
use XlsxGraft\Zone;

/**
 * XlsxReader against the real CLI: each value, the sheets, a sheet asked
 * for, refused or stopped, and the book's scope.
 */
final class XlsxReaderTest extends TestCase
{
    /**
     * A row of each kind of value, with an empty column and a row left out:
     * a shared string, an inline string, then a number, a boolean, an error
     * value, a formula's string and an ISO 8601 date-time, then a date, a
     * time, a date-time and elapsed time made of numbers by their formats.
     */
    private const ROWS = '<row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" t="inlineStr"><is><t>in&#9;line</t></is></c></row>'
        .'<row r="3"><c r="A3"><v>3</v></c><c r="B3" t="b"><v>1</v></c><c r="C3" t="e"><v>#N/A</v></c>'
        .'<c r="D3" t="str"><f>A3</f><v>formula</v></c><c r="E3" t="d"><v>2024-01-31T08:30:00</v></c></row>'
        .'<row r="4"><c r="A4" s="1"><v>45322</v></c><c r="B4" s="2"><v>0.5</v></c>'
        .'<c r="C4" s="3"><v>45322.25</v></c><c r="D4" s="4"><v>1.5</v></c></row>';

    private string $dir;

    protected function setUp(): void
    {
        Engine::cli();
        $this->dir = Engine::tempDir();
    }

    protected function tearDown(): void
    {
        Engine::removeDir($this->dir);
    }

    public function testReadsEachValueAsItsType(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', self::ROWS)]);
        $result = XlsxReader::open($path, static fn(Book $book): array => iterator_to_array($book->sheet()->rows(), false));

        self::assertEquals(Result::ok([
            new Row(1, ['a', null, "in\tline"]),
            new Row(3, [3.0, true, new CellError('#N/A'), 'formula', new \DateTimeImmutable('2024-01-31T08:30:00', new \DateTimeZone('UTC'))]),
            new Row(4, [new CalendarDate(2024, 1, 31), new TimeOfDay(12, 0, 0, 0), new \DateTimeImmutable('2024-01-31T06:00:00', new \DateTimeZone('UTC')), 129600.0]),
        ]), $result);
        // An elapsed time is a float even when it is whole seconds.
        self::assertSame(129600.0, $result->value[2]->values[3] ?? null);
    }

    public function testPlacesADateTimeInTheZoneGiven(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row><c t="d"><v>2024-01-31T08:30:00.25</v></c></row>')]);
        $tokyo = new \DateTimeZone('Asia/Tokyo');
        foreach (['Asia/Tokyo', $tokyo] as $timezone) {
            $result = XlsxReader::open($path, static fn(Book $book): mixed => iterator_to_array($book->sheet()->rows(), false)[0]->values[0], $timezone);

            self::assertEquals(new \DateTimeImmutable('2024-01-31T08:30:00.250', $tokyo), $result->value);
            self::assertInstanceOf(\DateTimeImmutable::class, $result->value);
            self::assertSame('Asia/Tokyo', $result->value->getTimezone()->getName());
        }

        $result = XlsxReader::open($path, static fn(Book $book): mixed => iterator_to_array($book->sheet()->rows(), false)[0]->values[0], '+09:00');
        self::assertEquals(new \DateTimeImmutable('2024-01-31T08:30:00.250+09:00'), $result->value);
    }

    public function testADateTimeTheZoneSkipsOrHasTwiceStopsItsSheet(): void
    {
        $rows = '<row r="1"><c t="d"><v>2024-03-10T01:59:00</v></c></row><row r="2"><c t="d"><v>2024-03-10T02:30:00</v></c></row>';
        $repeated = '<row r="5"><c t="d"><v>2024-11-03T01:30:00</v></c></row>';
        $path = Workbook::write($this->dir, [Workbook::worksheet('Skipped', $rows), Workbook::worksheet('Repeated', $repeated)]);
        $count = static fn(Book $book, string $which): int => iterator_count($book->sheet($which)->rows());

        $result = XlsxReader::open($path, static fn(Book $book): int => $count($book, 'Skipped'), 'America/New_York');
        self::assertTrue($result->failed());
        self::assertSame('datetime_unrepresentable', $result->error?->code);
        self::assertSame('row 2: 2024-03-10 02:30:00.000 cannot be placed in America/New_York: its daylight saving skips it', $result->error->message);
        self::assertSame([], $result->diagnostics);

        $result = XlsxReader::open($path, static fn(Book $book): int => $count($book, 'Repeated'), 'America/New_York');
        self::assertSame('row 5: 2024-11-03 01:30:00.000 cannot be placed in America/New_York: it has it twice', $result->error?->message);
    }

    public function testADateTimeOfAYearBelow101IsPlacedInItsOwnYear(): void
    {
        // 0024-03-10 02:30 is no gap: New York kept its local mean time then.
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row><c t="d"><v>0024-03-10T02:30:00</v></c></row>')]);
        $zone = new \DateTimeZone('America/New_York');
        $result = XlsxReader::open($path, static fn(Book $book): mixed => iterator_to_array($book->sheet()->rows(), false)[0]->values[0], $zone);

        self::assertEquals(new \DateTimeImmutable('0024-03-10 02:30:00', $zone), $result->value);
    }

    public function testListsTheSheetsAndTheirProblems(): void
    {
        $path = Workbook::write($this->dir, [
            Workbook::worksheet('Data', '<row><c t="s"><v>1</v></c></row>'),
            Workbook::chartsheet('Chart', 'veryHidden'),
            Workbook::unrelated('Lost', 'hidden'),
        ]);
        $listed = [];
        $result = XlsxReader::open($path, static function (Book $book) use (&$listed): void {
            $listed[] = $book->sheets();
            iterator_to_array($book->sheet('data')->rows());
            $listed[] = $book->sheets();
        });

        $sheets = [
            new SheetInfo(0, 'Data', SheetKind::Worksheet, SheetState::Visible),
            new SheetInfo(1, 'Chart', SheetKind::Chartsheet, SheetState::VeryHidden),
            new SheetInfo(2, 'Lost', SheetKind::Unknown, SheetState::Hidden),
        ];
        self::assertEquals([$sheets, $sheets], $listed);
        // The part not known is the workbook's error: the read fails with no
        // error, its problem found by both runs and kept once.
        self::assertEquals(Result::failure($result->diagnostics), $result);
        self::assertCount(1, $result->diagnostics);
        $problem = $result->diagnostics[0];
        self::assertSame(['sheet_relationship_missing', 'error'], [$problem->code, $problem->severity]);
        self::assertNotSame([], $problem->location);
    }

    public function testChoosesASheetByNameLetterCaseAsideOrByIndexOrTheFirst(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row r="1"/>'), Workbook::worksheet('-Other', '<row r="2"/>')]);
        $numbers = static fn(Book $book, string|int|null $which): array => array_map(
            static fn(Row $row): int => $row->number,
            iterator_to_array($which === null ? $book->sheet()->rows() : $book->sheet($which)->rows(), false),
        );

        $result = XlsxReader::open($path, static fn(Book $book): array => [$numbers($book, null), $numbers($book, '-oTHER'), $numbers($book, 1), $numbers($book, 0)]);
        self::assertEquals(Result::ok([[1], [2], [2], [1]]), $result);
    }

    public function testARefusedSheetReadsNothingAndStaysTheError(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row/>'), Workbook::chartsheet('Chart')]);
        $result = XlsxReader::open($path, static fn(Book $book): array => [
            iterator_to_array($book->sheet('none')->rows()),
            iterator_to_array($book->sheet(5)->rows()),
            iterator_to_array($book->sheet('Chart')->rows()),
            iterator_count($book->sheet('Data')->rows()),
        ]);

        self::assertTrue($result->failed());
        self::assertNull($result->value);
        self::assertSame(['sheet_not_found', 'no sheet is named "none"', 'error'], [$result->error?->code, $result->error?->message, $result->error?->severity]);
        self::assertSame([], $result->diagnostics);
    }

    public function testAProblemInASheetEndsItsRowsAfterTheRowsBefore(): void
    {
        $rows = '<row r="1"><c t="s"><v>1</v></c></row><row r="2"><c t="s"><v>9</v></c></row><row r="3"/>';
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', $rows)]);
        $read = [];
        $result = XlsxReader::open($path, static function (Book $book) use (&$read): void {
            foreach ($book->sheet()->rows() as $row) {
                $read[] = $row;
            }
        });

        self::assertEquals([new Row(1, ['b'])], $read);
        self::assertTrue($result->failed());
        self::assertNull($result->error);
        self::assertSame('shared_string_missing', $result->diagnostics[0]->code);
        self::assertSame(['index' => 9], $result->diagnostics[0]->args);
        self::assertSame(['part' => '/xl/sheets/sheet1.xml'], $result->diagnostics[0]->location);
    }

    public function testStoppingEarlyIsNoFailure(): void
    {
        $row = '<row><c t="inlineStr"><is><t>a long enough value</t></is></c></row>';
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', str_repeat($row, 50_000))]);
        $result = XlsxReader::open($path, static function (Book $book): array {
            $first = [];
            foreach ($book->sheet()->rows() as $row) {
                $first[] = $row->number;

                break;
            }
            // A generator dropped part way stops its CLI, as one left
            // standing does when the callback returns.
            $dropped = $book->sheet()->rows();
            $dropped->current();
            unset($dropped);
            $left = $book->sheet()->rows();
            $left->current();

            return [$first, $left];
        });

        self::assertTrue($result->success);
        self::assertSame([1], $result->value[0] ?? null);
        $this->expectException(ClosedException::class);
        $result->value[1]->next();
    }

    public function testAFileThatIsNoWorkbookListsNoSheet(): void
    {
        file_put_contents("{$this->dir}/book.xlsx", 'not a zip');
        $result = XlsxReader::open("{$this->dir}/book.xlsx", static fn(Book $book): array => [$book->sheets(), iterator_to_array($book->sheet()->rows())]);

        self::assertSame('workbook_unreadable', $result->error?->code);
        self::assertCount(1, $result->diagnostics);
    }

    public function testAFileThatWillNotOpenFailsTheCliRun(): void
    {
        $path = "{$this->dir}/missing.xlsx";
        $result = XlsxReader::open($path, static fn(Book $book): array => $book->sheets());

        self::assertTrue($result->failed());
        self::assertSame('exit_status', $result->error?->code);
        self::assertStringStartsWith("the CLI exited with status 1: xlsx-graft: failed to open {$path}: ", $result->error->message);
    }

    public function testAPathWithANulIsRefusedBeforeTheCallback(): void
    {
        $result = XlsxReader::open("book\0.xlsx", static fn(): never => self::fail('the callback ran'));

        self::assertSame(['invalid_arg', 'a path holds no NUL'], [$result->error?->code, $result->error?->message]);
    }

    public function testASheetNameThatIsNotUtf8IsRefused(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data')]);
        $result = XlsxReader::open($path, static fn(Book $book): array => [iterator_to_array($book->sheet("\xff")->rows()), iterator_to_array($book->sheet("a\0")->rows())]);

        self::assertSame('invalid_arg', $result->error?->code);
    }

    public function testReadsBytes(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row r="7"/>')]);
        $result = XlsxReader::openBytes((string) file_get_contents($path), static fn(Book $book): array => iterator_to_array($book->sheet()->rows(), false));

        self::assertEquals(Result::ok([new Row(7, [])]), $result);
    }

    public function testBytesGoToAFileOnlyTheUserMayReadRemovedAtTheClose(): void
    {
        $core = BookCore::ofBytes(Engine::cli(), 'bytes', new Zone(new \DateTimeZone('UTC')), $this->dir);
        self::assertInstanceOf(BookCore::class, $core);
        $files = glob("{$this->dir}/xlsx-graft-*.xlsx") ?: [];

        self::assertCount(1, $files);
        self::assertSame(['bytes', 0o600], [file_get_contents($files[0]), fileperms($files[0]) & 0o777]);
        $core->close();
        self::assertFileDoesNotExist($files[0]);
    }

    public function testBytesThatCannotBeWrittenAreIo(): void
    {
        $diagnostic = BookCore::ofBytes(Engine::cli(), 'x', new Zone(new \DateTimeZone('UTC')), "{$this->dir}/none");

        self::assertInstanceOf(Diagnostic::class, $diagnostic);
        self::assertSame('io', $diagnostic->code);
        self::assertStringStartsWith("the workbook could not be written to a temporary file (fopen({$this->dir}/none/xlsx-graft-", $diagnostic->message);
    }

    public function testTheCallersExceptionPropagatesTheBookClosedFirst(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data', '<row/><row/>')]);
        $kept = null;
        // An arrow function would capture $kept by value.
        $thrown = self::thrown(static function () use ($path, &$kept): void {
            XlsxReader::open($path, static function (Book $book) use (&$kept): never {
                $kept = $book;
                $book->sheet()->rows()->current();

                throw new \LogicException('the caller');
            });
        });

        self::assertEquals(new \LogicException('the caller'), $thrown);
        self::assertInstanceOf(Book::class, $kept);
        $book = $kept;
        $closed = new ClosedException("the book is closed: use it inside XlsxReader::open's callback");
        self::assertEquals($closed, self::thrown(static fn(): mixed => $book->sheets()));
        self::assertEquals($closed, self::thrown(static fn(): mixed => $book->sheet()));
    }

    public function testANegativeIndexAndAnUnknownZoneThrow(): void
    {
        $path = Workbook::write($this->dir, [Workbook::worksheet('Data')]);
        $thrown = self::thrown(static fn(): Result => XlsxReader::open($path, static fn(Book $book): mixed => $book->sheet(-1)));
        self::assertEquals(new \InvalidArgumentException('a sheet index is from 0, not -1'), $thrown);

        $this->expectException(\DateInvalidTimeZoneException::class);
        XlsxReader::open($path, static fn(): null => null, 'Nowhere/Else');
    }

    /** What `$call` throws; null when it returns. */
    private static function thrown(callable $call): ?\Throwable
    {
        try {
            $call();
        } catch (\Throwable $e) {
            return $e;
        }

        return null;
    }
}
