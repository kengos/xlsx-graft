<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Cli;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\Tests\Support\Engine;

final class CliTest extends TestCase
{
    private string $dir;

    protected function setUp(): void
    {
        $this->dir = Engine::tempDir();
    }

    protected function tearDown(): void
    {
        Engine::removeDir($this->dir);
    }

    public function testTheEnvironmentWinsOverThePathPassedAndPath(): void
    {
        $onPath = Engine::fakeCli($this->dir, 'exit 0');

        self::assertSame('/from/env', Cli::find('/given', ['XLSX_GRAFT_CLI' => '/from/env', 'PATH' => $this->dir]));
        self::assertSame('/given', Cli::find('/given', ['XLSX_GRAFT_CLI' => '', 'PATH' => $this->dir]));
        self::assertSame('/given', Cli::find('/given', ['PATH' => $this->dir]));
        self::assertSame($onPath, Cli::find('', ['XLSX_GRAFT_CLI' => '', 'PATH' => $this->dir]));
    }

    public function testPathIsSearchedInOrderForAnExecutableFile(): void
    {
        $plain = Engine::tempDir();
        $first = Engine::fakeCli($plain, 'exit 0', 'xlsx-graft', 0o644);
        mkdir("{$this->dir}/xlsx-graft");
        $exe = Engine::fakeCli($this->dir, 'exit 0', 'xlsx-graft.exe');

        try {
            $path = implode(PATH_SEPARATOR, ['', '/no/such/dir', $plain, $this->dir]);
            self::assertSame($exe, Cli::find(null, ['PATH' => $path]));
        } finally {
            rmdir("{$this->dir}/xlsx-graft");
            unlink($first);
            rmdir($plain);
        }
    }

    public function testReadsTheEnvironmentByName(): void
    {
        $cli = (string) getenv('XLSX_GRAFT_CLI');
        $path = (string) getenv('PATH');

        try {
            self::assertSame(['XLSX_GRAFT_CLI' => $cli, 'PATH' => $path], Cli::environment());
            putenv('XLSX_GRAFT_CLI');
            self::assertSame(['PATH' => $path], Cli::environment());
        } finally {
            putenv("XLSX_GRAFT_CLI={$cli}");
        }
    }

    public function testNoCliAnywhereThrowsWithTheThreeWaysToPointAtOne(): void
    {
        try {
            Cli::find(null, ['PATH' => $this->dir]);
            self::fail('no CLI was found, but nothing was thrown');
        } catch (CliNotFoundException $e) {
            self::assertStringStartsWith('no xlsx-graft CLI was found.', $e->getMessage());
            self::assertStringContainsString('never downloads', $e->getMessage());
            self::assertStringContainsString('set XLSX_GRAFT_CLI', $e->getMessage());
            self::assertStringContainsString("XlsxGraft::version('/path/to/xlsx-graft')", $e->getMessage());
            self::assertStringContainsString('put xlsx-graft on PATH', $e->getMessage());
        }

        $this->expectException(CliNotFoundException::class);
        Cli::find(null, []);
    }
}
