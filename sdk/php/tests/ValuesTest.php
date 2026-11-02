<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\CalendarDate;
use XlsxGraft\CellError;
use XlsxGraft\TimeOfDay;

final class ValuesTest extends TestCase
{
    public function testEachValueOfThePackagesOwnPrintsAsTheCliDoes(): void
    {
        self::assertSame('0099-01-31', (string) new CalendarDate(99, 1, 31));
        self::assertSame('08:30:00.005', (string) new TimeOfDay(8, 30, 0, 5));
        self::assertSame('#N/A', (string) new CellError('#N/A'));
    }
}
