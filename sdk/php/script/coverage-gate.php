<?php

declare(strict_types=1);

// The 100% line coverage bar, read from PHPUnit's clover report: PHPUnit has
// no fail-under of its own. Lists every uncovered line when it is red.
//
//     php script/coverage-gate.php <clover.xml>

$report = $argv[1] ?? '';
$xml = is_file($report) ? simplexml_load_file($report) : false;
if ($xml === false) {
    fwrite(STDERR, "coverage: no readable clover report at '{$report}'; run phpunit first\n");
    exit(1);
}

$metrics = $xml->xpath('/coverage/project/metrics');
if ($metrics === null || $metrics === false || $metrics === []) {
    fwrite(STDERR, "coverage: {$report} carries no project metrics\n");
    exit(1);
}

$statements = (int) $metrics[0]['statements'];
$covered = (int) $metrics[0]['coveredstatements'];
// A report that measured nothing (no coverage driver, or a <source> that
// matched no file) would otherwise read as 100%.
if ($statements === 0) {
    fwrite(STDERR, "coverage: {$report} measured no lines at all\n");
    exit(1);
}

if ($covered === $statements) {
    printf("coverage: %d/%d lines (100%%)\n", $covered, $statements);
    exit(0);
}

fprintf(STDERR, "coverage: %d/%d lines, %d uncovered; the bar is 100%%\n", $covered, $statements, $statements - $covered);
foreach ($xml->xpath('//file') ?: [] as $file) {
    $uncovered = [];
    foreach ($file->line as $line) {
        if ((string) $line['type'] === 'stmt' && (int) $line['count'] === 0) {
            $uncovered[] = (string) $line['num'];
        }
    }
    if ($uncovered !== []) {
        fprintf(STDERR, "  %s: %s\n", (string) $file['name'], implode(', ', $uncovered));
    }
}
exit(1);
