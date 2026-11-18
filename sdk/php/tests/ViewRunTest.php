<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use XlsxGraft\Cli;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\Diagnostic;
use XlsxGraft\Grid;
use XlsxGraft\Result;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Tree;
use XlsxGraft\Viewer;
use XlsxGraft\ViewReport;
use XlsxGraft\ViewRun;

/**
 * What a Viewer makes of the arguments it is given, the file it writes and
 * the exit statuses and output the real CLI does not produce, from fake CLIs
 * and {@see ViewRun}'s `$dir`.
 */
final class ViewRunTest extends TestCase
{
    private const TREE = '{"ok":true,"error":null,"diagnostics":[],"complete":true,"relationshipsKnown":true,"relationships":[],"parts":[]}';

    private const GRID = '{"ok":true,"error":null,"diagnostics":[],"sheet":null,"dimension":null,"used":null,"shown":null,'
        .'"truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"rows":[],"merges":[]}';

    private string $cli;

    private string $dir;

    protected function setUp(): void
    {
        $this->cli = Engine::cli();
        $this->dir = Engine::tempDir();
    }

    protected function tearDown(): void
    {
        putenv(Cli::ENV_VAR.'='.$this->cli);
        Engine::removeDir($this->dir);
    }

    public function testTheArgumentsOfATreeAndOfADescribeSheetTheCliGets(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; echo '.escapeshellarg(self::GRID));
        $options = ['--sheet=-x', '--range=A1:C3', '--max-rows=2'];
        $grid = ViewRun::path($cli, ViewRun::DESCRIBE_SHEET, $options, 'book.xlsx');

        self::assertTrue($grid->success);
        self::assertSame("describe-sheet\n--format\njson\n--sheet=-x\n--range=A1:C3\n--max-rows=2\n--\nbook.xlsx\n", file_get_contents("{$this->dir}/args"));

        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; echo '.escapeshellarg(self::TREE));
        $tree = ViewRun::path($cli, ViewRun::TREE, [], '-book.xlsx');

        self::assertTrue($tree->success);
        self::assertSame("tree\n--format\njson\n--\n-book.xlsx\n", file_get_contents("{$this->dir}/args"));
    }

    public function testAViewerPassesOnlyTheValuesItIsGiven(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; echo '.escapeshellarg(self::GRID));
        putenv(Cli::ENV_VAR.'=');
        $viewer = new Viewer(cli: $cli);
        $viewer->describeSheet('book.xlsx');
        self::assertSame("describe-sheet\n--format\njson\n--\nbook.xlsx\n", file_get_contents("{$this->dir}/args"));

        $viewer->describeSheet('book.xlsx', sheet: 'Data', range: 'A1:H50', maxRows: 20, maxColumns: 5);
        self::assertSame("describe-sheet\n--format\njson\n--sheet=Data\n--range=A1:H50\n--max-rows=20\n--max-columns=5\n--\nbook.xlsx\n", file_get_contents("{$this->dir}/args"));

        $viewer->describeSheetBytes('x', sheet: 0, maxRows: 0);
        self::assertStringContainsString("\n--sheet-index=0\n--max-rows=0\n--\n", (string) file_get_contents("{$this->dir}/args"));

        $viewer->describeSheetBytes('x', sheet: '7');
        self::assertStringContainsString("\n--sheet=7\n--\n", (string) file_get_contents("{$this->dir}/args"));
    }

    public function testBytesAreWrittenToAFileTheCliReadsAndRemovedAfter(): void
    {
        $body = 'for last; do :; done; cp "$last" "$(dirname "$0")/data"; ls -ld "$(dirname "$last")" | cut -c1-10 > "$(dirname "$0")/mode"; echo '.escapeshellarg(self::TREE);
        $result = ViewRun::bytes(Engine::fakeCli($this->dir, $body), ViewRun::TREE, [], 'bytes', $this->dir);

        self::assertTrue($result->success);
        self::assertSame(['bytes', "drwx------\n"], [file_get_contents("{$this->dir}/data"), file_get_contents("{$this->dir}/mode")]);
        self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
    }

    public function testTheFileIsRemovedWhenTheCliCannotBeRunOrRefusesAnOption(): void
    {
        try {
            ViewRun::bytes("{$this->dir}/none", ViewRun::TREE, [], 'bytes', $this->dir);
            self::fail('no CliNotFoundException');
        } catch (CliNotFoundException) {
            self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
        }
        $cli = Engine::fakeCli($this->dir, 'echo "xlsx-graft: no" >&2; exit 2');
        try {
            ViewRun::bytes($cli, ViewRun::DESCRIBE_SHEET, [], 'bytes', $this->dir);
            self::fail('no InvalidArgumentException');
        } catch (\InvalidArgumentException) {
            self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
        }
    }

    public function testANulIsInvalidArgAndTheCliIsNotRun(): void
    {
        $cli = Engine::fakeCli($this->dir, 'touch "$(dirname "$0")/called"; echo '.escapeshellarg(self::TREE));
        $path = ViewRun::path($cli, ViewRun::TREE, [], "book\0.xlsx");
        $option = ViewRun::bytes($cli, ViewRun::DESCRIBE_SHEET, ["--sheet=a\0"], 'bytes', $this->dir);
        $range = ViewRun::path($cli, ViewRun::DESCRIBE_SHEET, ["--range=A1\0"], 'book.xlsx');

        self::assertEquals(Result::failure([], new Diagnostic('invalid_arg', 'a path holds no NUL')), $path);
        self::assertEquals(Result::failure([], new Diagnostic('invalid_arg', 'an argument holds no NUL')), $option);
        self::assertEquals(Result::failure([], new Diagnostic('invalid_arg', 'an argument holds no NUL')), $range);
        self::assertFileDoesNotExist("{$this->dir}/called");
        self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
    }

    public function testAFileThatCannotBeWrittenIsIoAndTheCliIsNotRun(): void
    {
        $cli = Engine::fakeCli($this->dir, 'touch "$(dirname "$0")/called"; echo '.escapeshellarg(self::TREE));
        $none = "{$this->dir}/none";
        $result = ViewRun::bytes($cli, ViewRun::TREE, [], 'bytes', $none);

        self::assertNull($result->value);
        self::assertSame('io', $result->error?->code);
        self::assertStringStartsWith("the workbook could not be written to a temporary file ({$none}/xlsx-graft-", $result->error->message);
        self::assertFileDoesNotExist("{$this->dir}/called");
    }

    private static function what(string $view): string
    {
        return $view === ViewRun::TREE ? 'parts of the workbook' : 'cells of the sheet';
    }

    /** @return iterable<string, array{'tree'|'describe-sheet', string, string, string}> */
    public static function endings(): iterable
    {
        foreach ([ViewRun::TREE, ViewRun::DESCRIBE_SHEET] as $view) {
            yield "{$view}: exit 3 is exit_status" => [$view, 'echo oops >&2; exit 3', 'exit_status', 'the CLI exited with status 3: oops'];
            yield "{$view}: a signal is exit_status" => [$view, 'kill -9 $$', 'exit_status', 'the CLI was killed by signal 9'];
            yield "{$view}: exit 0 with no JSON is invalid_output" => [$view, 'echo nope', 'invalid_output', 'the CLI printed no JSON object with a boolean "ok" and the '.self::what($view).': nope'];
            yield "{$view}: exit 0 with no ok is invalid_output" => [$view, 'echo "{}"', 'invalid_output', 'the CLI printed no JSON object with a boolean "ok" and the '.self::what($view).': {}'];
            yield "{$view}: exit 0 with an object that lacks the view's keys is invalid_output" => [
                $view,
                'echo \'{"ok":true,"error":null,"diagnostics":[]}\'',
                'invalid_output',
                'the CLI printed no JSON object with a boolean "ok" and the '.self::what($view).': {"ok":true,"error":null,"diagnostics":[]}',
            ];
            yield "{$view}: exit 1 with an object that lacks the view's keys is exit_status" => [$view, 'echo \'{"ok":false,"error":null,"diagnostics":[]}\'; exit 1', 'exit_status', 'the CLI exited with status 1'];
            yield "{$view}: exit 1 with no JSON is exit_status" => [$view, 'echo "xlsx-graft: failed" >&2; exit 1', 'exit_status', 'the CLI exited with status 1: xlsx-graft: failed'];
        }
        yield 'tree: exit 2 is exit_status' => [ViewRun::TREE, 'echo "error: unexpected argument" >&2; exit 2', 'exit_status', 'the CLI exited with status 2: error: unexpected argument'];
        yield 'tree: exit 2 with an object is exit_status' => [ViewRun::TREE, 'echo '.escapeshellarg(self::TREE).'; exit 2', 'exit_status', 'the CLI exited with status 2'];
    }

    /** @param 'tree'|'describe-sheet' $view */
    #[DataProvider('endings')]
    public function testARunOutsideTheContractIsAFailedRunWithAnErrorAndNoValue(string $view, string $body, string $code, string $message): void
    {
        $result = ViewRun::path(Engine::fakeCli($this->dir, $body), $view, [], 'book.xlsx');

        self::assertEquals(Result::failure([], new Diagnostic($code, $message)), $result);
        self::assertNull($result->value);
    }

    /** @return iterable<string, array{string, string}> */
    public static function refusals(): iterable
    {
        yield 'the CLI prefix is removed and the text trimmed' => ["echo ' xlsx-graft: --range \"A0\" is not an A1 range' >&2; exit 2", '--range "A0" is not an A1 range'];
        yield 'clap text stays as it is' => ['echo "error: invalid value for --sheet-index" >&2; exit 2', 'error: invalid value for --sheet-index'];
        yield 'only the first prefix is removed' => ['echo "xlsx-graft: xlsx-graft: x" >&2; exit 2', 'xlsx-graft: x'];
        yield 'a prefix alone is no reason' => ['echo "xlsx-graft: " >&2; exit 2', ViewReport::NO_REASON];
        yield 'no stderr is no reason' => ['exit 2', ViewReport::NO_REASON];
        yield 'an object on stdout does not matter' => ['echo '.escapeshellarg(self::GRID).'; echo "xlsx-graft: refused" >&2; exit 2', 'refused'];
    }

    #[DataProvider('refusals')]
    public function testExit2OfDescribeSheetThrowsInvalidArgumentExceptionWithTheCliStderr(string $body, string $message): void
    {
        $cli = Engine::fakeCli($this->dir, $body);
        try {
            ViewRun::path($cli, ViewRun::DESCRIBE_SHEET, [], 'book.xlsx');
            self::fail('no InvalidArgumentException');
        } catch (\InvalidArgumentException $e) {
            self::assertSame($message, $e->getMessage());
        }
    }

    public function testTheFixedReasonIsTheSentenceTheDocsPromise(): void
    {
        self::assertSame('the CLI refused an argument (exit status 2) and gave no reason', ViewReport::NO_REASON);
    }

    public function testACliThatIsNotThereThrowsWhenTheViewerIsMadeOrWhenItIsRun(): void
    {
        putenv(Cli::ENV_VAR.'=');
        // A given path is not looked at until the CLI is run.
        $viewer = new Viewer(cli: '/no/such/xlsx-graft');
        try {
            $viewer->tree('book.xlsx');
            self::fail('no CliNotFoundException');
        } catch (CliNotFoundException) {
            $this->addToAssertionCount(1);
        }

        $path = getenv('PATH');
        putenv("PATH={$this->dir}");
        try {
            new Viewer();
            self::fail('no CliNotFoundException');
        } catch (CliNotFoundException) {
            $this->addToAssertionCount(1);
        } finally {
            putenv("PATH={$path}");
        }
    }

    public function testAFailedViewKeepsItsValueAndAnOkOneKeepsItsDiagnostics(): void
    {
        $diagnostic = '{"code":"part_unreadable","severity":"error","message":"m","args":{"part":"/p"},"location":{"part":"/p"},"decidedBy":null,"fix":null}';
        $failed = str_replace('"ok":true,"error":null,"diagnostics":[]', '"ok":false,"error":{"code":"io","message":"gone"},"diagnostics":['.$diagnostic.']', self::TREE);
        $expected = new Diagnostic('part_unreadable', 'm', 'error', ['part' => '/p'], ['part' => '/p']);

        foreach ([0, 1] as $status) {
            $result = ViewRun::path(Engine::fakeCli($this->dir, 'echo '.escapeshellarg($failed)."; exit {$status}"), ViewRun::TREE, [], 'book.xlsx');
            self::assertEquals(Result::failure([$expected], new Diagnostic('io', 'gone'), new Tree(true, true, [], [])), $result);
        }
        $notFound = str_replace('"ok":true,"error":null', '"ok":false,"error":{"code":"sheet_not_found","message":"no"}', self::GRID);
        $grid = ViewRun::path(Engine::fakeCli($this->dir, 'echo '.escapeshellarg($notFound).'; exit 1'), ViewRun::DESCRIBE_SHEET, [], 'book.xlsx');

        self::assertSame('sheet_not_found', $grid->error?->code);
        self::assertInstanceOf(Grid::class, $grid->value);
        self::assertNull($grid->value->sheet);

        $ok = str_replace('"diagnostics":[]', '"diagnostics":['.$diagnostic.']', self::TREE);
        $result = ViewRun::path(Engine::fakeCli($this->dir, 'echo '.escapeshellarg($ok)), ViewRun::TREE, [], 'book.xlsx');
        self::assertEquals(Result::ok(new Tree(true, true, [], []), [$expected]), $result);
    }
}
