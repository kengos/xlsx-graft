<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * No engine CLI could be found, or the one found cannot be run, PHP's own
 * setup included (proc_open() disabled, no temporary file).
 *
 * A configuration mistake, not a failed call, so it is thrown rather than
 * returned as a {@see Result}.
 */
final class CliNotFoundException extends XlsxGraftException {}
