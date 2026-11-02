<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use XlsxGraft\Book;
use XlsxGraft\Cli;
use XlsxGraft\Diagnostic;
use XlsxGraft\Result;
use XlsxGraft\Row;
use XlsxGraft\SheetInfo;
use XlsxGraft\SheetKind;
use XlsxGraft\SheetState;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\XlsxReader;

/**
 * What a book makes of the runs the real CLI never gives (docs/sdk.md §
 * Listing sheets and § Reading rows), from fake CLIs printing them.
 */
final class CliRunsTest extends TestCase
{
    private const END = '{"ok":true,"sheet":{"index":0,"name":"Data"},"rows":1,"error":null,"diagnostics":[]}';

    private string $cli;

    private string $dir;

    /** @var list<Row> the rows the last read gave */
    private array $rows = [];

    protected function setUp(): void
    {
        $this->cli = Engine::cli();
        $this->dir = Engine::tempDir();
        putenv(Cli::ENV_VAR.'=');
    }

    protected function tearDown(): void
    {
        putenv(Cli::ENV_VAR.'='.$this->cli);
        Engine::removeDir($this->dir);
    }

    public function testAnEndLineIsTheResult(): void
    {
        $diagnostic = '{"code":"x_y","severity":"warning","message":"m","args":{"n":1},"location":{"part":"/p"},"decidedBy":{"rule":{"id":"r"}},"fix":null,"origin":"o"}';
        $end = '{"ok":false,"sheet":null,"rows":0,"error":{"code":"sheet_not_found","message":"no sheet"},"diagnostics":['.$diagnostic.']}';
        $result = $this->read(self::lines('{"row":2,"cells":[{"type":"number","value":3}]}', $end).'exit 1');

        self::assertEquals(Result::failure(
            [new Diagnostic('x_y', 'm', 'warning', ['n' => 1], ['part' => '/p'], ['rule' => ['id' => 'r']])],
            new Diagnostic('sheet_not_found', 'no sheet'),
        ), $result);
        self::assertEquals([new Row(2, [3.0])], $this->rows);
    }

    public function testAStatusOtherThan0Or1OrASignalIsExitStatus(): void
    {
        $result = $this->read(self::lines('{"row":1,"cells":[]}', self::END).'echo "xlsx-graft: broken" >&2; exit 3');
        self::assertEquals(Result::failure([], new Diagnostic('exit_status', 'the CLI exited with status 3: xlsx-graft: broken')), $result);

        $result = $this->read(self::lines('{"row":1,"cells":[]}').'kill -9 $$');
        self::assertSame('the CLI was killed by signal 9', $result->error?->message);

        // A signal after the end line is still a failure.
        $result = $this->read(self::lines(self::END).'kill -9 $$');
        self::assertSame('the CLI was killed by signal 9', $result->error?->message);

        // Exit 1 with no end line is the host failing.
        $result = $this->read('echo "xlsx-graft: failed to read" >&2; exit 1');
        self::assertSame('the CLI exited with status 1: xlsx-graft: failed to read', $result->error?->message);
    }

    /** @return iterable<string, array{string, string}> */
    public static function invalidOutput(): iterable
    {
        yield 'not JSON' => [self::lines('{"row":1,"cells":[]}', 'oops', self::END), 'oops'];
        yield 'no end line' => [self::lines('{"row":1,"cells":[]}'), '{"row":1,"cells":[]}'];
        yield 'nothing' => ['', ''];
        yield 'a line after the end' => [self::lines(self::END, '{"row":2,"cells":[]}'), '{"row":2,"cells":[]}'];
        yield 'an end without rows' => [self::lines('{"ok":true,"sheet":null,"error":null,"diagnostics":[]}'), '{"ok":true,"sheet":null,"error":null,"diagnostics":[]}'];
        yield 'a diagnostic without fix' => [
            self::lines('{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":{},"location":{},"decidedBy":null}]}'),
            '{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":{},"location":{},"decidedBy":null}]}',
        ];
        yield 'args that are no object' => [
            self::lines('{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":[],"location":{},"decidedBy":null,"fix":null}]}'),
            '{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":[],"location":{},"decidedBy":null,"fix":null}]}',
        ];
        yield 'an error that is no object' => [self::lines('{"ok":false,"sheet":null,"rows":0,"error":"x","diagnostics":[]}'), '{"ok":false,"sheet":null,"rows":0,"error":"x","diagnostics":[]}'];
        yield 'ok that is no boolean' => [self::lines('{"ok":1,"sheet":null,"rows":0,"error":null,"diagnostics":[]}'), '{"ok":1,"sheet":null,"rows":0,"error":null,"diagnostics":[]}'];
        yield 'diagnostics that are no array' => [self::lines('{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":{}}'), '{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":{}}'];
        yield 'a row number that is no integer' => [self::lines('{"row":"1","cells":[]}', self::END), '{"row":"1","cells":[]}'];
        yield 'a line that is no object' => [self::lines('[1]', self::END), '[1]'];
        foreach (['empty' => '1', 'string' => '1', 'number' => '"3"', 'boolean' => '1', 'date' => '"2024-1-31"', 'time' => '"12:00:00"', 'datetime' => '"2024-01-31 12:00:00.000"', 'elapsed' => '1.5', 'error' => 'null'] as $type => $value) {
            $line = "{\"row\":1,\"cells\":[{\"type\":\"{$type}\",\"value\":{$value}}]}";
            yield "a {$type} of another type" => [self::lines($line, self::END), $line];
        }
    }

    #[DataProvider('invalidOutput')]
    public function testOutputOutOfTheContractIsInvalidOutput(string $body, string $quoted): void
    {
        $result = $this->read($body.'exit 0');

        $expected = 'the CLI printed no JSON object with the end line last'.($quoted === '' ? '' : ": {$quoted}");
        self::assertEquals(Result::failure([], new Diagnostic('invalid_output', $expected)), $result);
    }

    public function testAValueOfAnUnknownTypeStopsTheSheet(): void
    {
        $result = $this->read(self::lines('{"row":1,"cells":[]}', '{"row":4,"cells":[{"type":"future","value":1}]}', '{"row":5,"cells":[]}', self::END).'exit 0');

        self::assertEquals(Result::failure([], new Diagnostic('unknown_value', 'row 4: the CLI returned a value of type future, which this package does not know')), $result);
        self::assertEquals([new Row(1, [])], $this->rows);
    }

    public function testStoppingEarlyStopsTheCliAndIsNoFailureWhateverItEnds(): void
    {
        // Rows for as long as stdout takes them. PHP ignores SIGPIPE, and so
        // does a child it starts, so the loop ends on the write that fails,
        // as the real CLI does.
        $forever = 'while printf \'%s\\n\' \'{"row":1,"cells":[]}\'; do :; done';
        foreach ([$forever, self::lines('{"row":1,"cells":[]}').'exit 3', self::lines('{"row":1,"cells":[]}').'kill -9 $$'] as $body) {
            $cli = Engine::fakeCli($this->dir, $body);
            $result = XlsxReader::open('book.xlsx', static function (Book $book): int {
                foreach ($book->sheet()->rows() as $row) {
                    return $row->number;
                }

                return 0;
            }, cli: $cli);

            self::assertEquals(Result::ok(1), $result, $body);
        }
    }

    public function testADiagnosticIsKeptAsOftenAsOneRunGaveIt(): void
    {
        $d = static fn(string $code): string => "{\"code\":\"{$code}\",\"severity\":\"notice\",\"message\":\"m\",\"args\":{},\"location\":{},\"decidedBy\":null,\"fix\":null}";
        $listing = '{"ok":true,"sheets":[],"error":null,"diagnostics":['.$d('a').','.$d('a').']}';
        $end = '{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":['.$d('a').','.$d('b').','.$d('a').','.$d('a').']}';
        $cli = Engine::fakeCli($this->dir, 'if [ "$1" = sheets ]; then '.self::lines($listing).'else '.self::lines($end).'fi');
        $result = XlsxReader::open('book.xlsx', static fn(Book $book): int => count($book->sheets()) + iterator_count($book->sheet()->rows()), cli: $cli);

        self::assertTrue($result->success);
        self::assertSame(['a', 'a', 'b', 'a'], array_map(static fn(Diagnostic $d): string => $d->code, $result->diagnostics));
    }

    public function testTheSheetsOfAListing(): void
    {
        $listing = '{"ok":true,"sheets":[{"index":0,"name":"A","kind":"worksheet","state":"visible"},{"index":1,"name":"B","kind":"future","state":"faded"}],"error":null,"diagnostics":[]}';
        $result = $this->list(self::lines($listing).'exit 0');

        self::assertEquals(Result::ok([
            new SheetInfo(0, 'A', SheetKind::Worksheet, SheetState::Visible),
            new SheetInfo(1, 'B', SheetKind::Unknown, SheetState::Unknown),
        ]), $result);
    }

    public function testAListingOutOfTheContractOrAFailedRunListsNoSheet(): void
    {
        $result = $this->list(self::lines('{"ok":true,"sheets":[1],"error":null,"diagnostics":[]}').'exit 0');
        self::assertEquals(Result::failure([], new Diagnostic('invalid_output', 'the CLI printed no JSON object with the sheets: {"ok":true,"sheets":[1],"error":null,"diagnostics":[]}')), $result);

        $result = $this->list('echo "xlsx-graft: failed to open" >&2; exit 1');
        self::assertEquals(Result::failure([], new Diagnostic('exit_status', 'the CLI exited with status 1: xlsx-graft: failed to open')), $result);

        $result = $this->list(self::lines('{"ok":true,"sheets":[],"error":null,"diagnostics":[]}').'kill -9 $$');
        self::assertSame('exit_status', $result->error?->code);
    }

    /**
     * Reads the first sheet's rows from a fake CLI whose `read` runs
     * `$body`.
     *
     * @return Result<null>
     */
    private function read(string $body): Result
    {
        $cli = Engine::fakeCli($this->dir, "[ \"\$1\" = read ] || exit 99\n{$body}");
        $this->rows = [];

        return XlsxReader::open('book.xlsx', function (Book $book): null {
            foreach ($book->sheet()->rows() as $row) {
                $this->rows[] = $row;
            }

            return null;
        }, cli: $cli);
    }

    /**
     * The sheets of a fake CLI whose `sheets` runs `$body`.
     *
     * @return Result<list<SheetInfo>>
     */
    private function list(string $body): Result
    {
        $cli = Engine::fakeCli($this->dir, "[ \"\$1\" = sheets ] || exit 99\n{$body}");

        return XlsxReader::open('book.xlsx', static fn(Book $book): array => $book->sheets(), cli: $cli);
    }

    /** A shell command per line, each printing it. */
    private static function lines(string ...$lines): string
    {
        return implode('', array_map(static fn(string $line): string => 'printf \'%s\n\' '.escapeshellarg($line)."\n", $lines));
    }
}
