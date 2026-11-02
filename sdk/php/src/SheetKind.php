<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * What kind of sheet a {@see SheetInfo} is. Only a worksheet has rows to
 * read.
 */
enum SheetKind: string
{
    case Worksheet = 'worksheet';
    case Chartsheet = 'chartsheet';
    case Dialogsheet = 'dialogsheet';
    case Macrosheet = 'macrosheet';
    /** Which part the sheet is is not known, or a CLI newer than this package names a kind it does not know. */
    case Unknown = 'unknown';
}
