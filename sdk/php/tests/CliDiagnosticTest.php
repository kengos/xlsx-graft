<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\CliDiagnostic;

final class CliDiagnosticTest extends TestCase
{
    public function testExitStatusQuotesStderr(): void
    {
        $diagnostic = CliDiagnostic::exitStatus(2, null, "xlsx-graft: error: unexpected argument\n\nUsage: ...\n");

        self::assertSame('exit_status', $diagnostic->code);
        self::assertSame('the CLI exited with status 2: xlsx-graft: error: unexpected argument Usage: ...', $diagnostic->message);
        self::assertSame('the CLI exited with status 1', CliDiagnostic::exitStatus(1, null, " \n")->message);
        self::assertSame('the CLI was killed by signal 9', CliDiagnostic::exitStatus(-1, 9, '')->message);
    }

    public function testInvalidOutputQuotesStdout(): void
    {
        $diagnostic = CliDiagnostic::invalidOutput('a string "version"', "0.0.0\n");

        self::assertSame('invalid_output', $diagnostic->code);
        self::assertSame('the CLI printed no JSON object with a string "version": 0.0.0', $diagnostic->message);
        self::assertSame('the CLI printed no JSON object with a string "version"', CliDiagnostic::invalidOutput('a string "version"', '')->message);
    }

    public function testCutsTheQuoteInCharacters(): void
    {
        $reason = 'the CLI exited with status 1: ';

        self::assertSame($reason.str_repeat('x', 200), CliDiagnostic::exitStatus(1, null, str_repeat('x', 200))->message);
        self::assertSame($reason.str_repeat('x', 200).'...', CliDiagnostic::exitStatus(1, null, str_repeat('x', 201))->message);
        self::assertSame($reason.str_repeat('表', 200).'...', CliDiagnostic::exitStatus(1, null, str_repeat('表', 250))->message);
    }

    public function testKeepsTheQuoteOnOneLine(): void
    {
        self::assertSame('the CLI exited with status 1: a b c d', CliDiagnostic::exitStatus(1, null, "a\r\nb\u{2028}c\u{2029}d\u{202E}\t")->message);
    }

    public function testKeepsPrintableAsciiOfWhatIsNotUtf8(): void
    {
        self::assertSame('the CLI exited with status 1: bad bytes here', CliDiagnostic::exitStatus(1, null, "bad\xff\xfebytes\x00here")->message);
    }
}
