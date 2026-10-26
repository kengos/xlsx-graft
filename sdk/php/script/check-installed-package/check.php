<?php

declare(strict_types=1);

// Run by `make sdk:verify:php` after it installs the archived zip into this
// project, copied out of the checkout: loads the package from there, checks
// what the zip holds, and that it reaches the engine on the gated line's PHP.
//
//     php check.php <this project's dir> <the zip> <engine/Cargo.toml> <PHP release>

use XlsxGraft\XlsxGraft;

[, $project, $zip, $cargoToml, $release] = ($argv ?? []) + array_fill(0, 5, '');
$installed = "{$project}/vendor/kengos/xlsx-graft";

/** @phpstan-assert true $ok */
function check(bool $ok, string $message): void
{
    if (!$ok) {
        fwrite(STDERR, $message."\n");
        exit(1);
    }
}

require "{$project}/vendor/autoload.php";

preg_match('/^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms', (string) file_get_contents($cargoToml), $match);
$engine = $match[1] ?? '';
check($engine !== '', "{$cargoToml} has no [workspace.package] version");

check(PHP_VERSION === $release, 'the package was loaded on PHP '.PHP_VERSION.", not on the gated {$release}");

$loaded = (string) (new ReflectionClass(XlsxGraft::class))->getFileName();
check($loaded === "{$installed}/src/XlsxGraft.php", "loaded XlsxGraft from {$loaded}, not from {$installed}");

// The zip is what Packagist would serve: the sources, composer.json, the
// README and the license files, and nothing else.
$archive = new ZipArchive();
check($archive->open($zip) === true, "{$zip} does not open as a zip");
$shipped = [];
for ($i = 0; $i < $archive->numFiles; $i++) {
    $name = (string) $archive->getNameIndex($i);
    if (!str_ends_with($name, '/')) {
        $shipped[] = $name;
    }
}
$expected = ['LICENSE-APACHE', 'LICENSE-BSD', 'LICENSE-MIT', 'README.md', 'composer.json'];
$unexpected = array_filter($shipped, static fn(string $file): bool => !in_array($file, $expected, true) && preg_match('#^src/[^/]+\.php$#', $file) !== 1);
check($unexpected === [], 'the zip ships '.implode(', ', $unexpected));
foreach ([...$expected, 'src/XlsxGraft.php'] as $file) {
    check(in_array($file, $shipped, true), "the zip ships no {$file}");
}

$manifest = json_decode((string) $archive->getFromName('composer.json'), true);
check(is_array($manifest), "{$zip}'s composer.json is not a JSON object");
$version = $manifest['version'] ?? null;
check($version === $engine, 'kengos/xlsx-graft is '.json_encode($version).", the engine is {$engine}");
$require = $manifest['require'] ?? null;
check(is_array($require) && array_keys($require) === ['php'], 'kengos/xlsx-graft requires '.json_encode($require).', not php alone');
check(!isset($manifest['scripts']), 'kengos/xlsx-graft runs scripts: '.json_encode($manifest['scripts'] ?? null));

// Composer installed the package and nothing else beside it.
$lock = json_decode((string) file_get_contents("{$project}/composer.lock"), true);
$packages = is_array($lock) ? ($lock['packages'] ?? null) : null;
$names = is_array($packages) ? array_column($packages, 'name') : [];
check($names === ['kengos/xlsx-graft'], 'installed '.json_encode($names).', not kengos/xlsx-graft alone');

$result = XlsxGraft::version();
check($result->success, 'version() failed: '.json_encode($result->diagnostics));
check($result->value === $engine, "version() is {$result->value}, the engine is {$engine}");

echo "installed kengos/xlsx-graft {$engine} from {$installed} runs engine {$result->value} on PHP {$release}\n";
