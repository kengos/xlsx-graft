<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Whether a {@see SheetInfo} shows.
 */
enum SheetState: string
{
    case Visible = 'visible';
    /** Hidden, and the user may show it. */
    case Hidden = 'hidden';
    /** Hidden, and only a program may show it. */
    case VeryHidden = 'veryHidden';
    /** A CLI newer than this package names a state it does not know. */
    case Unknown = 'unknown';
}
