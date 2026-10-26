<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Cli;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\Result;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\XlsxGraft;

final class XlsxGraftTest extends TestCase
{
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

    public function testReturnsTheEngineVersionFromTheCli(): void
    {
        self::assertEquals(Result::ok(Engine::version()), XlsxGraft::version());
        self::assertSame(Engine::version(), XlsxGraft::version('/elsewhere/xlsx-graft')->value);
    }

    public function testRunsTheCliPassedWhenXlsxGraftCliIsUnset(): void
    {
        putenv(Cli::ENV_VAR.'=');

        self::assertSame(Engine::version(), XlsxGraft::version($this->cli)->value);
        $this->expectException(CliNotFoundException::class);
        XlsxGraft::version('/elsewhere/xlsx-graft');
    }

    public function testANonZeroExitIsAFailedResult(): void
    {
        putenv(Cli::ENV_VAR.'=');
        $result = XlsxGraft::version(Engine::fakeCli($this->dir, 'echo "xlsx-graft: broken" >&2; exit 4'));

        self::assertTrue($result->failed());
        self::assertNull($result->value);
        self::assertSame('exit_status', $result->diagnostics[0]->code);
        self::assertSame('the CLI exited with status 4: xlsx-graft: broken', $result->diagnostics[0]->message);
    }

    public function testACliKilledByASignalIsAFailedResult(): void
    {
        putenv(Cli::ENV_VAR.'=');
        $result = XlsxGraft::version(Engine::fakeCli($this->dir, 'kill -9 $$'));

        self::assertSame('exit_status', $result->diagnostics[0]->code);
        self::assertSame('the CLI was killed by signal 9', $result->diagnostics[0]->message);
    }

    public function testNoOutputIsAFailedResult(): void
    {
        putenv(Cli::ENV_VAR.'=');
        $result = XlsxGraft::version(Engine::fakeCli($this->dir, 'exit 0'));

        self::assertTrue($result->failed());
        self::assertSame('invalid_output', $result->diagnostics[0]->code);
        self::assertSame('the CLI printed no JSON object with a string "version"', $result->diagnostics[0]->message);
    }

    public function testOutputThatIsNotUtf8IsAFailedResult(): void
    {
        putenv(Cli::ENV_VAR.'=');
        $result = XlsxGraft::version(Engine::fakeCli($this->dir, "printf '{\"version\":\"\\377\"}'"));

        self::assertSame('invalid_output', $result->diagnostics[0]->code);
        self::assertSame('the CLI printed no JSON object with a string "version": {"version":" "}', $result->diagnostics[0]->message);
    }

    public function testOutputWithNoStringVersionIsAFailedResult(): void
    {
        putenv(Cli::ENV_VAR.'=');

        foreach (['0.0.0', '["0.0.0"]', '{"version":1}', '{"name":"0.0.0"}', '"0.0.0"', '{"version":"0.0.0"}{"x":1}'] as $stdout) {
            $cli = Engine::fakeCli($this->dir, "printf '%s' '{$stdout}'");
            $result = XlsxGraft::version($cli);

            self::assertTrue($result->failed(), $stdout);
            self::assertSame('invalid_output', $result->diagnostics[0]->code, $stdout);
            self::assertStringEndsWith(": {$stdout}", $result->diagnostics[0]->message);
        }
    }
}
