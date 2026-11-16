<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\TemporaryFile;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Tests\Support\RecordingStream;

/**
 * TemporaryFile: the file and the directory it makes, their removal, and each
 * way the write fails.
 */
final class TemporaryFileTest extends TestCase
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

    public function testWritesTheDataToAFileInADirectoryOnlyTheUserMayEnter(): void
    {
        $path = TemporaryFile::write($this->dir, '.json', 'data');

        self::assertMatchesRegularExpression('#^'.preg_quote($this->dir, '#').'/xlsx-graft-[0-9a-f]{16}/xlsx-graft\.json$#', $path);
        self::assertSame('data', file_get_contents($path));
        self::assertSame(0o700, fileperms(dirname($path)) & 0o777);
        TemporaryFile::remove($path);
    }

    public function testRemoveDeletesTheFileAndTheDirectoryAndASecondRemoveRaisesNoWarning(): void
    {
        $path = TemporaryFile::write($this->dir, '.json', 'data');
        TemporaryFile::remove($path);
        TemporaryFile::remove($path);

        self::assertSame([], glob("{$this->dir}/*"));
    }

    public function testAMissingDirectoryThrowsWithTheFolderAndTheReason(): void
    {
        try {
            TemporaryFile::write("{$this->dir}/none", '.json', 'data');
            self::fail('no RuntimeException');
        } catch (\RuntimeException $e) {
            self::assertStringStartsWith("{$this->dir}/none/xlsx-graft-", $e->getMessage());
            self::assertStringContainsString('mkdir(): No such file or directory', $e->getMessage());
        }
    }

    public function testAFolderNameThatExistsThrowsAndNothingIsOpenedOrRemoved(): void
    {
        $e = $this->thrownWithStream('mkdir');

        self::assertSame(substr(RecordingStream::$calls[0], strlen('mkdir ')).': mkdir(): File exists', $e->getMessage());
        self::assertCount(1, RecordingStream::$calls);
        self::assertStringStartsWith('mkdir '.RecordingStream::SCHEME.'://dir/xlsx-graft-', RecordingStream::$calls[0]);
    }

    public function testAFileThatCannotBeOpenedThrowsAndOnlyTheFolderIsRemoved(): void
    {
        $e = $this->thrownWithStream('stream_open');

        self::assertStringContainsString('Failed to open stream', $e->getMessage());
        self::assertSame(['mkdir', 'stream_open', 'rmdir'], $this->operations());
        self::assertSame('mkdir '.$this->folderOf(RecordingStream::$calls[2]), RecordingStream::$calls[0]);
    }

    public function testAShortWriteThrowsAndTheFileAndTheFolderAreRemoved(): void
    {
        $e = $this->thrownWithStream('stream_write');

        self::assertSame('a short write', $e->getMessage());
        self::assertSame(['mkdir', 'stream_open', 'stream_write', 'unlink', 'rmdir'], $this->operations());
        self::assertSame('stream_open '.$this->folderOf(RecordingStream::$calls[4]).'/xlsx-graft.json', RecordingStream::$calls[1]);
        self::assertSame('unlink '.$this->folderOf(RecordingStream::$calls[4]).'/xlsx-graft.json', RecordingStream::$calls[3]);
    }

    public function testTheProcessUmaskIsNotChangedWhileTheFileIsOpened(): void
    {
        // Set to a value other than 0o077, which TemporaryFile::write before
        // the fix of #168 set while it opened the file, so that the test
        // fails for that code whatever the umask of the environment.
        $before = umask(0o022);
        RecordingStream::register(null);
        try {
            $path = TemporaryFile::write(RecordingStream::SCHEME.'://dir', '.json', 'data');
            TemporaryFile::remove($path);
        } finally {
            RecordingStream::unregister();
            $after = umask($before);
        }

        self::assertSame([0o022], RecordingStream::$umasks);
        self::assertSame(0o022, $after);
        self::assertSame(['mkdir', 'stream_open', 'stream_write', 'unlink', 'rmdir'], $this->operations());
    }

    /** The exception of a write to the stream wrapper that fails `$operation`. */
    private function thrownWithStream(string $operation): \RuntimeException
    {
        RecordingStream::register($operation);
        try {
            TemporaryFile::write(RecordingStream::SCHEME.'://dir', '.json', 'data');
        } catch (\RuntimeException $e) {
            return $e;
        } finally {
            RecordingStream::unregister();
        }
        self::fail('no RuntimeException');
    }

    /** @return list<string> the operation of each recorded call */
    private function operations(): array
    {
        return array_map(static fn(string $call): string => explode(' ', $call)[0], RecordingStream::$calls);
    }

    /** The path after the operation in a recorded call `$call` of mkdir or rmdir. */
    private function folderOf(string $call): string
    {
        return explode(' ', $call, 2)[1];
    }
}
