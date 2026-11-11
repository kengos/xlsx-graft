<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Writes bytes to a file that only this user may read, for the CLI to read by
 * its path (docs/sdk.md § Running the CLI), and removes it.
 *
 * @internal
 */
final class TemporaryFile
{
    /**
     * Writes `$data` to a new file named `xlsx-graft-<random>` followed by
     * `$suffix` in `$dir`, and returns its path. A file written in part is
     * removed.
     *
     * @throws \RuntimeException when the file cannot be written, its message the reason
     */
    public static function write(string $dir, string $suffix, string $data): string
    {
        $path = $dir.DIRECTORY_SEPARATOR.'xlsx-graft-'.bin2hex(random_bytes(8)).$suffix;
        error_clear_last();
        // Made 0600 rather than changed to it after, when another user could
        // have opened it already. PHP runs on one thread, so the process's
        // umask is this call's for as long as it is set.
        $mask = umask(0o077);
        try {
            $file = @fopen($path, 'x');
        } finally {
            umask($mask);
        }
        $written = $file !== false && fwrite($file, $data) === strlen($data);
        $file === false || fclose($file);
        if (!$written) {
            $why = error_get_last()['message'] ?? 'a short write';
            @unlink($path);

            throw new \RuntimeException($why);
        }

        return $path;
    }

    /** Removes the file at `$path`, if it is there. */
    public static function remove(string $path): void
    {
        @unlink($path);
    }
}
