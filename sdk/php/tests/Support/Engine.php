<?php

declare(strict_types=1);

namespace XlsxGraft\Tests\Support;

use XlsxGraft\Cli;

/**
 * What the tests share: the engine's CLI, the engine's version, and fake CLIs
 * for the runs the real one never gives.
 */
final class Engine
{
    /** sdk/php, the package's root. */
    public const PACKAGE_ROOT = __DIR__.'/../..';

    /** The CLI `make engine:cli-bin` built; the gate points XLSX_GRAFT_CLI at it. */
    public static function cli(): string
    {
        $path = getenv(Cli::ENV_VAR);
        if ($path === false || $path === '') {
            throw new \RuntimeException(Cli::ENV_VAR.' is unset; run the tests through make sdk:test:php');
        }

        return $path;
    }

    /** The engine workspace's version, from engine/Cargo.toml. */
    public static function version(): string
    {
        $cargoToml = (string) file_get_contents(self::PACKAGE_ROOT.'/../../engine/Cargo.toml');
        if (preg_match('/^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms', $cargoToml, $found) !== 1) {
            throw new \RuntimeException('engine/Cargo.toml has no [workspace.package] version');
        }

        return $found[1];
    }

    /** A directory of its own under the system's temporary directory. */
    public static function tempDir(): string
    {
        $dir = sys_get_temp_dir().'/xlsx-graft-test-'.bin2hex(random_bytes(8));
        mkdir($dir);

        return $dir;
    }

    /**
     * Writes `$name` into `$dir`: a shell script running `$body`, with `$mode`.
     */
    public static function fakeCli(string $dir, string $body, string $name = 'xlsx-graft', int $mode = 0o755): string
    {
        $path = "{$dir}/{$name}";
        file_put_contents($path, "#!/bin/sh\n{$body}\n");
        chmod($path, $mode);

        return $path;
    }

    public static function removeDir(string $dir): void
    {
        foreach (glob("{$dir}/*") ?: [] as $file) {
            unlink($file);
        }
        rmdir($dir);
    }
}
