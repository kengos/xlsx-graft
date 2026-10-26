<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Diagnostic;
use XlsxGraft\Result;

final class ResultTest extends TestCase
{
    public function testOkCarriesTheValueAndItsDiagnostics(): void
    {
        $note = new Diagnostic('note', 'for people');
        $result = Result::ok('0.0.0', [$note]);

        self::assertTrue($result->success);
        self::assertFalse($result->failed());
        self::assertSame('0.0.0', $result->value);
        self::assertSame([$note], $result->diagnostics);
        self::assertSame([], Result::ok('0.0.0')->diagnostics);
    }

    public function testFailureCarriesNoValue(): void
    {
        $why = new Diagnostic('exit_status', 'the CLI exited with status 1');
        $result = Result::failure([$why]);

        self::assertFalse($result->success);
        self::assertTrue($result->failed());
        self::assertNull($result->value);
        self::assertSame('exit_status', $result->diagnostics[0]->code);
        self::assertSame('the CLI exited with status 1', $result->diagnostics[0]->message);
    }
}
