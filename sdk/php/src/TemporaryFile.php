<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Writes bytes to a file in a directory that only this user may enter, for the
 * CLI to read by its path (docs/sdk.md § Running the CLI), and removes both.
 *
 * @internal
 */
final class TemporaryFile
{
    /**
     * Makes the directory `xlsx-graft-<random>` in `$dir`, writes `$data` to a
     * new file named `xlsx-graft` followed by `$suffix` in it, and returns the
     * file's path. When the write fails, it removes only what it made: the file
     * if it was opened, then the directory.
     *
     * @throws \RuntimeException when the file cannot be written, its message the reason
     */
    public static function write(string $dir, string $suffix, string $data): string
    {
        $folder = $dir.DIRECTORY_SEPARATOR.'xlsx-graft-'.bin2hex(random_bytes(8));
        error_clear_last();
        // mkdir fails for a name that exists, so `write` never uses or removes
        // what another user made. Mode 0700 is only lowered by the umask, never
        // raised, so only this user may enter the directory whatever the mode
        // of the file in it. The process's umask is left alone, so a file
        // another thread creates (PHP built with ZTS) is not affected. The file
        // is not created with fopen and then chmod'ed to 0600 with the umask
        // left alone: a descriptor another user opened on the file before the
        // chmod could still read what is written later. On Windows mkdir
        // ignores the mode, and the directory takes the access list of `$dir`.
        if (!@mkdir($folder, 0o700)) {
            throw new \RuntimeException("{$folder}: ".(error_get_last()['message'] ?? 'mkdir failed'));
        }
        $path = $folder.DIRECTORY_SEPARATOR.'xlsx-graft'.$suffix;
        $file = @fopen($path, 'x');
        $written = $file !== false && fwrite($file, $data) === strlen($data);
        $file === false || fclose($file);
        if (!$written) {
            $why = error_get_last()['message'] ?? 'a short write';
            $file === false || @unlink($path);
            @rmdir($folder);

            throw new \RuntimeException($why);
        }

        return $path;
    }

    /**
     * Removes the file at `$path`, as `write` returned it, and the directory
     * `write` made for it, if they exist.
     */
    public static function remove(string $path): void
    {
        @unlink($path);
        @rmdir(dirname($path));
    }
}
