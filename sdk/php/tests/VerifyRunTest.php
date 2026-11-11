<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use XlsxGraft\Cli;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\Diagnostic;
use XlsxGraft\Result;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Verifier;
use XlsxGraft\VerifyRun;

/**
 * What a Verifier makes of the arguments it is given, the files it writes and
 * the exit statuses and output the real CLI does not produce, from fake CLIs
 * and {@see VerifyRun}'s `$dir`.
 */
final class VerifyRunTest extends TestCase
{
    private const OK = '{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"error":null,"diagnostics":[]}';

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

    public function testTheArgumentsAndThePolicyFileTheCliGets(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; cp "${4#--policy=}" "$(dirname "$0")/policy"; echo '.escapeshellarg(self::OK));
        $rules = ['--target=-x', '--target=excel 2016', '--fail-on=warn'];
        $result = VerifyRun::path($cli, '{"maxEntries":1}', 'book.xlsx', $rules, $this->dir);

        self::assertEquals(Result::ok(null), $result);
        $args = explode("\n", rtrim((string) file_get_contents("{$this->dir}/args"), "\n"));
        self::assertSame(['verify', '--format', 'json'], array_slice($args, 0, 3));
        self::assertMatchesRegularExpression('#^--policy='.preg_quote($this->dir, '#').'/xlsx-graft-[0-9a-f]{16}\.json$#', $args[3]);
        self::assertSame(['--target=-x', '--target=excel 2016', '--fail-on=warn', '--', 'book.xlsx'], array_slice($args, 4));
        self::assertSame('{"maxEntries":1}', file_get_contents("{$this->dir}/policy"));
        self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
    }

    public function testNoPolicyIsNoPolicyArgument(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; echo '.escapeshellarg(self::OK));
        VerifyRun::path($cli, null, 'book.xlsx', [], $this->dir);

        self::assertSame("verify\n--format\njson\n--\nbook.xlsx\n", file_get_contents("{$this->dir}/args"));
    }

    public function testBytesAreWrittenToAFileTheCliReadsAndRemovedAfter(): void
    {
        $body = 'for last; do :; done; cp "$last" "$(dirname "$0")/data"; ls -l "$last" | cut -c1-10 > "$(dirname "$0")/mode"; echo '.escapeshellarg(self::OK);
        $result = VerifyRun::bytes(Engine::fakeCli($this->dir, $body), '{}', 'bytes', [], $this->dir);

        self::assertTrue($result->success);
        self::assertSame(['bytes', "-rw-------\n"], [file_get_contents("{$this->dir}/data"), file_get_contents("{$this->dir}/mode")]);
        self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
    }

    public function testTheFilesAreRemovedWhenTheCliCannotBeRun(): void
    {
        try {
            VerifyRun::bytes("{$this->dir}/none", '{}', 'bytes', [], $this->dir);
            self::fail('no CliNotFoundException');
        } catch (CliNotFoundException) {
            self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
        }
    }

    public function testANulIsInvalidArgAndTheCliIsNotRun(): void
    {
        $cli = Engine::fakeCli($this->dir, 'touch "$(dirname "$0")/called"; echo '.escapeshellarg(self::OK));
        $path = VerifyRun::path($cli, null, "book\0.xlsx", [], $this->dir);
        $rule = VerifyRun::bytes($cli, '{}', 'bytes', ["--target=a\0"], $this->dir);

        self::assertEquals(Result::failure([], new Diagnostic('invalid_arg', 'a path holds no NUL')), $path);
        self::assertEquals(Result::failure([], new Diagnostic('invalid_arg', 'an argument holds no NUL')), $rule);
        self::assertFileDoesNotExist("{$this->dir}/called");
        self::assertSame([], glob("{$this->dir}/xlsx-graft-*"));
    }

    public function testFilesThatCannotBeWrittenAreIoAndTheCliIsNotRun(): void
    {
        $cli = Engine::fakeCli($this->dir, 'touch "$(dirname "$0")/called"; echo '.escapeshellarg(self::OK));
        $none = "{$this->dir}/none";
        $data = VerifyRun::bytes($cli, null, 'bytes', [], $none);
        $policy = VerifyRun::path($cli, '{}', 'book.xlsx', [], $none);

        self::assertSame('io', $data->error?->code);
        self::assertStringStartsWith("the workbook could not be written to a temporary file (fopen({$none}/xlsx-graft-", $data->error->message);
        self::assertSame('io', $policy->error?->code);
        self::assertStringStartsWith("the policy could not be written to a temporary file (fopen({$none}/xlsx-graft-", $policy->error->message);
        self::assertFileDoesNotExist("{$this->dir}/called");
    }

    /** @return iterable<string, array{string, string, string}> */
    public static function endings(): iterable
    {
        yield 'exit 3 is exit_status' => ['echo oops >&2; exit 3', 'exit_status', 'the CLI exited with status 3: oops'];
        yield 'a signal is exit_status' => ['kill -9 $$', 'exit_status', 'the CLI was killed by signal 9'];
        yield 'exit 0 with no JSON is invalid_output' => ['echo nope', 'invalid_output', 'the CLI printed no JSON object with a boolean "ok": nope'];
        yield 'exit 0 with no ok is invalid_output' => ['echo "{}"', 'invalid_output', 'the CLI printed no JSON object with a boolean "ok": {}'];
        yield 'exit 1 with no JSON is exit_status' => ['echo "xlsx-graft: failed" >&2; exit 1', 'exit_status', 'the CLI exited with status 1: xlsx-graft: failed'];
        yield 'exit 2 with an object with no ok is exit_status' => ['echo "{}"; exit 2', 'exit_status', 'the CLI exited with status 2'];
        yield 'exit 2 with no stdout is exit_status' => ['echo "error: unexpected argument" >&2; exit 2', 'exit_status', 'the CLI exited with status 2: error: unexpected argument'];
    }

    #[DataProvider('endings')]
    public function testARunOutsideTheContractIsAFailedRunWithAnError(string $body, string $code, string $message): void
    {
        $result = VerifyRun::path(Engine::fakeCli($this->dir, $body), null, 'book.xlsx', [], $this->dir);

        self::assertEquals(Result::failure([], new Diagnostic($code, $message)), $result);
    }

    public function testExit1And2WithTheObjectAreResultsAndTheErrorIsTakenOnlyWhenNotOk(): void
    {
        $diagnostic = '{"code":"part_denied","severity":"error","message":"m","args":{"kind":"macro"},"location":{"part":"/p"},"decidedBy":null,"fix":null}';
        $notOk = '{"ok":false,"failOn":"error","counts":{"error":1,"warn":0,"notice":0},"error":{"code":"io","message":"gone"},"diagnostics":['.$diagnostic.']}';
        $ok = '{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"error":{"code":"io","message":"ignored"},"diagnostics":['.$diagnostic.']}';
        $expected = new Diagnostic('part_denied', 'm', 'error', ['kind' => 'macro'], ['part' => '/p']);

        foreach ([1, 2] as $status) {
            $result = VerifyRun::path(Engine::fakeCli($this->dir, 'echo '.escapeshellarg($notOk)."; exit {$status}"), null, 'book.xlsx', [], $this->dir);
            self::assertEquals(Result::failure([$expected], new Diagnostic('io', 'gone')), $result);
        }
        $result = VerifyRun::path(Engine::fakeCli($this->dir, 'echo '.escapeshellarg($ok)), null, 'book.xlsx', [], $this->dir);
        self::assertEquals(Result::ok(null, [$expected]), $result);
    }

    public function testAnArrayOfRulesHoldsStringsOnly(): void
    {
        $verifier = new Verifier();
        // Not strings, which the signature's types cannot tell PHPStan.
        /** @var array<string> $notStrings */
        $notStrings = ['a', 1];
        foreach (['targets', 'bestEffort', 'allowFunctions', 'allowParts'] as $name) {
            try {
                $verifier->verify('book.xlsx', ...[$name => $notStrings]);
                self::fail("{$name} took an int");
            } catch (\InvalidArgumentException $e) {
                self::assertSame("{$name} holds a value that is not a string", $e->getMessage());
            }
        }
        $this->expectException(\InvalidArgumentException::class);
        $verifier->verifyBytes('', targets: $notStrings);
    }

    public function testEachValueOfAnArrayOfRulesIsOneArgumentWhateverItsKey(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s\n" "$@" > "$(dirname "$0")/args"; echo '.escapeshellarg(self::OK));
        putenv(Cli::ENV_VAR.'=');
        $verifier = new Verifier(cli: $cli);
        $result = $verifier->verify('book.xlsx', targets: ['k' => 'a', 5 => 'b'], allowParts: ['macro'], failOn: 'warn');

        self::assertTrue($result->success);
        self::assertSame("verify\n--format\njson\n--target=a\n--target=b\n--allow-part=macro\n--fail-on=warn\n--\nbook.xlsx\n", file_get_contents("{$this->dir}/args"));

        $verifier->verifyBytes('x', bestEffort: ['excel 2016'], allowFunctions: ['F']);
        self::assertStringContainsString("--best-effort=excel 2016\n--allow-function=F\n--\n", (string) file_get_contents("{$this->dir}/args"));
    }
}
