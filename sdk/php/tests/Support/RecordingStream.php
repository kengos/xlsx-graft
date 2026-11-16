<?php

declare(strict_types=1);

namespace XlsxGraft\Tests\Support;

/**
 * A stream wrapper that records each mkdir, stream_open, unlink and rmdir call
 * as "<operation> <path>" and each stream_write call as "stream_write" alone (a
 * write has no path), the umask of each stream_open call in `$umasks`, and
 * fails the one operation named in `$fails`, for the failures of
 * TemporaryFile that a real directory cannot give: a name that already
 * exists, a file that cannot be opened and a write that stores no bytes.
 */
final class RecordingStream
{
    public const SCHEME = 'xlsxgrafttest';

    /** @var list<string> */
    public static array $calls = [];

    /** @var list<int> the process's umask at each stream_open call */
    public static array $umasks = [];

    /** The operation to fail: 'mkdir', 'stream_open' or 'stream_write'; null fails none. */
    public static ?string $fails = null;

    /** @var resource|null set by PHP */
    public $context;

    public static function register(?string $fails): void
    {
        self::$calls = [];
        self::$umasks = [];
        self::$fails = $fails;
        stream_wrapper_register(self::SCHEME, self::class);
    }

    public static function unregister(): void
    {
        stream_wrapper_unregister(self::SCHEME);
    }

    public function mkdir(string $path, int $mode, int $options): bool
    {
        self::$calls[] = "mkdir {$path}";
        if (self::$fails === 'mkdir') {
            trigger_error('mkdir(): File exists', E_USER_WARNING);

            return false;
        }

        return true;
    }

    public function stream_open(string $path, string $mode, int $options, ?string &$openedPath): bool
    {
        self::$calls[] = "stream_open {$path}";
        self::$umasks[] = umask();

        return self::$fails !== 'stream_open';
    }

    public function stream_write(string $data): int
    {
        self::$calls[] = 'stream_write';

        return self::$fails === 'stream_write' ? 0 : strlen($data);
    }

    public function unlink(string $path): bool
    {
        self::$calls[] = "unlink {$path}";

        return true;
    }

    public function rmdir(string $path, int $options): bool
    {
        self::$calls[] = "rmdir {$path}";

        return true;
    }
}
