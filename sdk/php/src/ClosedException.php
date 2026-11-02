<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * A {@see Book} or its rows used after the callback of
 * {@see XlsxReader::open()} returned, which closed it. A mistake in the code
 * calling this package, so it is thrown rather than returned in a
 * {@see Result}.
 */
final class ClosedException extends XlsxGraftException {}
