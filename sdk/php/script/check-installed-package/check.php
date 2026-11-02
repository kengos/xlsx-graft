<?php

declare(strict_types=1);

// Run by `make sdk:verify:php` after it installs the archived zip into this
// project, copied out of the checkout: loads the package from there, checks
// what the zip holds, and that it reaches the engine on the gated line's PHP.
//
//     php check.php <this project's dir> <the zip> <engine/Cargo.toml> <PHP release>

use XlsxGraft\Book;
use XlsxGraft\Row;
use XlsxGraft\XlsxGraft;
use XlsxGraft\XlsxReader;

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

// A workbook of one sheet, Data, whose one cell is the string "x", read
// through the installed package: `sheets` and `read` both run.
$ns = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main';
$relationships = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
$rels = static fn(string $id, string $type, string $target): string => '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    ."<Relationship Id=\"{$id}\" Type=\"{$relationships}/{$type}\" Target=\"{$target}\"/></Relationships>";
$book = "{$project}/book.xlsx";
$written = new ZipArchive();
check($written->open($book, ZipArchive::CREATE | ZipArchive::OVERWRITE) === true, "{$book} cannot be written");
$written->addFromString('[Content_Types].xml', '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>');
$written->addFromString('_rels/.rels', $rels('rId1', 'officeDocument', 'xl/workbook.xml'));
$written->addFromString('xl/workbook.xml', "<workbook xmlns=\"{$ns}\" xmlns:r=\"{$relationships}\"><sheets><sheet name=\"Data\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>");
$written->addFromString('xl/_rels/workbook.xml.rels', $rels('rId1', 'worksheet', 'sheet1.xml'));
$written->addFromString('xl/sheet1.xml', "<worksheet xmlns=\"{$ns}\"><sheetData><row><c t=\"inlineStr\"><is><t>x</t></is></c></row></sheetData></worksheet>");
check($written->close(), "{$book} cannot be written");
$read = XlsxReader::open($book, static fn(Book $book): array => [
    array_column($book->sheets(), 'name'),
    array_map(static fn(Row $row): array => $row->values, iterator_to_array($book->sheet()->rows(), false)),
]);
check($read->success && $read->value === [['Data'], [['x']]], 'XlsxReader::open read '.json_encode([$read->value, $read->error, $read->diagnostics]));

echo "installed kengos/xlsx-graft {$engine} from {$installed} runs engine {$result->value} and reads a sheet on PHP {$release}\n";
