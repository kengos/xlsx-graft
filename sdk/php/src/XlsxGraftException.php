<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * The root of what this package throws.
 *
 * A call into the engine never throws; it returns a {@see Result}. What
 * throws is a setup that cannot reach the engine.
 */
class XlsxGraftException extends \RuntimeException {}
