<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\CliProcess;
use XlsxGraft\Tests\Support\Engine;

final class CliProcessTest extends TestCase
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

    public function testReturnsTheStatusStdoutAndStderrApart(): void
    {
        $cli = Engine::fakeCli($this->dir, 'printf "%s|" "$@"; echo oops >&2; exit 3');

        self::assertSame([3, null, '--version|a b|$HOME|', "oops\n"], CliProcess::run($cli, ['--version', 'a b', '$HOME']));
    }

    public function testClosesStdin(): void
    {
        // `cat` would wait for ever on an open stdin.
        $cli = Engine::fakeCli($this->dir, 'cat; echo done');

        self::assertSame([0, null, "done\n", ''], CliProcess::run($cli, []));
    }

    public function testDrainsAStderrLargerThanAPipeHolds(): void
    {
        $cli = Engine::fakeCli($this->dir, 'head -c 200000 /dev/zero >&2; echo out');

        [$status, $signal, $stdout, $stderr] = CliProcess::run($cli, []);
        self::assertSame([0, null, "out\n", 200000], [$status, $signal, $stdout, strlen($stderr)]);
    }

    public function testWaitsForTheStatusOfACliThatClosesStdoutFirst(): void
    {
        $cli = Engine::fakeCli($this->dir, 'exec >&-; sleep 0.2; exit 5');

        self::assertSame([5, null, '', ''], CliProcess::run($cli, []));
    }

    public function testTellsASignalFromAStatus(): void
    {
        $killed = Engine::fakeCli($this->dir, 'kill -9 $$');
        $exited = Engine::fakeCli($this->dir, 'exit 9', 'exits-9');

        self::assertSame([-1, 9, '', ''], CliProcess::run($killed, []));
        self::assertSame([9, null, '', ''], CliProcess::run($exited, []));
    }

    public function testAPhpWithProcOpenDisabledThrows(): void
    {
        self::assertSame(
            'this PHP cannot run the CLI: proc_open() is disabled (disable_functions)',
            self::runInChildPhp('disable_functions=proc_open'),
        );
    }

    public function testAPhpWithNoTemporaryFileThrows(): void
    {
        self::assertSame(
            'this PHP cannot run the CLI: no temporary file can take its stderr (sys_temp_dir: /nonexistent)',
            self::runInChildPhp('sys_temp_dir=/nonexistent'),
        );
    }

    /**
     * Runs CliProcess::run in a PHP started with `-d $ini`, and returns the
     * message of the CliNotFoundException it throws there.
     */
    private static function runInChildPhp(string $ini): string
    {
        $code = 'require $argv[1]; try { XlsxGraft\CliProcess::run("/bin/true", []); echo "ran"; } catch (XlsxGraft\CliNotFoundException $e) { echo $e->getMessage(); }';
        [$status, , $stdout, $stderr] = CliProcess::run(PHP_BINARY, ['-d', $ini, '-r', $code, Engine::PACKAGE_ROOT.'/vendor/autoload.php']);
        self::assertSame([0, ''], [$status, $stderr]);

        return $stdout;
    }

    public function testANameWithNoDirectoryIsLookedUpOnPathBySystem(): void
    {
        self::assertSame([0, null, "on path\n", ''], CliProcess::run('sh', ['-c', 'echo on path']));
        $this->expectException(CliNotFoundException::class);
        CliProcess::run('no-such-xlsx-graft', []);
    }

    public function testACliThatCannotBeRunThrows(): void
    {
        $plain = Engine::fakeCli($this->dir, 'exit 0', 'xlsx-graft', 0o644);

        foreach ([$plain, "{$this->dir}/missing", $this->dir] as $cli) {
            try {
                CliProcess::run($cli, ['--version']);
                self::fail("{$cli} ran");
            } catch (CliNotFoundException $e) {
                self::assertStringStartsWith("{$cli} could not be run (", $e->getMessage());
                self::assertStringContainsString('set XLSX_GRAFT_CLI', $e->getMessage());
            }
        }
    }
}
