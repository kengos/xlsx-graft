<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Finds the engine's CLI (`xlsx-graft`, built from engine/cli). PHP has no FFI
 * it can count on, so this package runs the CLI rather than loading the C
 * ABI's library.
 *
 * Looked up in this order, and the first one set wins (a set path that cannot
 * be run throws when it is run, rather than falls through; a set name with no
 * directory in it is looked up on `PATH` by the system):
 *
 * 1. the `XLSX_GRAFT_CLI` environment variable
 * 2. the path the caller passed
 * 3. `xlsx-graft` on `PATH`
 *
 * The environment comes first because where the engine lives is the
 * deployment's decision, and it has to be able to override application code.
 * Nothing here downloads anything.
 *
 * @internal
 */
final class Cli
{
    public const ENV_VAR = 'XLSX_GRAFT_CLI';

    public const COMMAND = 'xlsx-graft';

    /**
     * The CLI `$env` or `$given` names, or the first on `$env`'s `PATH`.
     *
     * @param array<string, string> $env the variables {@see self::environment()} reads
     *
     * @throws CliNotFoundException when none is set and none is on `PATH`
     */
    public static function find(?string $given, array $env): string
    {
        // An empty value counts as unset, as it does for the other SDKs.
        foreach ([$env[self::ENV_VAR] ?? null, $given] as $set) {
            if ($set !== null && $set !== '') {
                return $set;
            }
        }

        return self::onPath($env['PATH'] ?? '')
            ?? throw new CliNotFoundException(self::installHint('no xlsx-graft CLI was found'));
    }

    /**
     * `XLSX_GRAFT_CLI` and `PATH`, each read by name: getenv() with no name
     * leaves out what PHP-FPM passes (its `env[...]` and fastcgi_param), and
     * Windows spells PATH as Path, which only a lookup by name ignores.
     *
     * @return array<string, string>
     */
    public static function environment(): array
    {
        return array_filter([self::ENV_VAR => getenv(self::ENV_VAR), 'PATH' => getenv('PATH')], is_string(...));
    }

    /**
     * Why the CLI cannot be reached, followed by the three ways to point this
     * package at one.
     */
    public static function installHint(string $reason): string
    {
        return $reason.".\n\n"
            ."This package never downloads the engine. Point it at one of these ways:\n"
            .'  * set '.self::ENV_VAR." to the path of an xlsx-graft executable\n"
            ."  * pass XlsxGraft::version('/path/to/xlsx-graft')\n"
            .'  * put '.self::COMMAND.' on PATH';
    }

    /**
     * Searched by hand rather than left to the operating system, so the
     * environment can be handed in (the tests do), and so a CLI that is not
     * there is reported as missing rather than as a failed run. Windows'
     * `.exe` spelling is tried at every entry, on every system. An entry
     * outside open_basedir is skipped without its warning, which an
     * application may turn into an exception.
     */
    private static function onPath(string $path): ?string
    {
        foreach (explode(PATH_SEPARATOR, $path) as $dir) {
            foreach ([self::COMMAND, self::COMMAND.'.exe'] as $name) {
                $candidate = $dir.DIRECTORY_SEPARATOR.$name;
                if ($dir !== '' && @is_file($candidate) && @is_executable($candidate)) {
                    return $candidate;
                }
            }
        }

        return null;
    }
}
