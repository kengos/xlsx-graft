<?php

declare(strict_types=1);

namespace XlsxGraft\Tests\Support;

/**
 * Workbooks for the tests to read, built as a test asks: a package with a
 * shared strings table of "a" and "b", styles whose cell formats 1 to 4 are
 * a date, a time, a date-time and an elapsed time, and the sheets the test
 * gives. The same workbook as engine/cli/tests/bin/workbook.rs builds,
 * written again in PHP.
 */
final class Workbook
{
    public const NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main';

    private const TYPES = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';

    /**
     * A worksheet named `$name` whose sheetData holds `$rows`.
     *
     * @return array{string, string|null, string|null, string} the name, its state (null: none written),
     *                                                       the type of its relationship (null: its r:id names
     *                                                       none) and its part's XML
     */
    public static function worksheet(string $name, string $rows = '', ?string $state = null): array
    {
        return [$name, $state, self::TYPES.'/worksheet', '<worksheet xmlns="'.self::NS."\"><sheetData>{$rows}</sheetData></worksheet>"];
    }

    /** @return array{string, string|null, string|null, string} */
    public static function chartsheet(string $name, ?string $state = null): array
    {
        return [$name, $state, self::TYPES.'/chartsheet', '<chartsheet xmlns="'.self::NS.'"/>'];
    }

    /**
     * A sheet named `$name` whose r:id names no relationship.
     *
     * @return array{string, string|null, string|null, string}
     */
    public static function unrelated(string $name, ?string $state = null): array
    {
        return [$name, $state, null, ''];
    }

    /**
     * Writes a workbook of `$sheets`, in order, each at
     * xl/sheets/sheet<n>.xml, to `$dir`/book.xlsx, and returns its path.
     *
     * @param list<array{string, string|null, string|null, string}> $sheets
     */
    public static function write(string $dir, array $sheets): string
    {
        $listed = '';
        $related = self::rel('rIdS', self::TYPES.'/sharedStrings', 'sst.xml').self::rel('rIdT', self::TYPES.'/styles', 'styles.xml');
        $parts = [];
        foreach ($sheets as $i => [$name, $state, $type, $xml]) {
            $n = $i + 1;
            $listed .= "<sheet name=\"{$name}\" sheetId=\"{$n}\"".($state === null ? '' : " state=\"{$state}\"")." r:id=\"rId{$n}\"/>";
            if ($type !== null) {
                $related .= self::rel("rId{$n}", $type, "sheets/sheet{$n}.xml");
                $parts["xl/sheets/sheet{$n}.xml"] = $xml;
            }
        }
        $formats = implode('', array_map(static fn(int $id): string => "<xf numFmtId=\"{$id}\"/>", [0, 14, 21, 22, 46]));
        $entries = [
            '[Content_Types].xml' => '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                .'<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                .'<Default Extension="xml" ContentType="application/xml"/></Types>',
            '_rels/.rels' => self::rels(self::rel('rId1', self::TYPES.'/officeDocument', 'xl/workbook.xml')),
            'xl/workbook.xml' => '<workbook xmlns="'.self::NS.'" xmlns:r="'.self::TYPES."\"><sheets>{$listed}</sheets></workbook>",
            'xl/_rels/workbook.xml.rels' => self::rels($related),
            'xl/sst.xml' => '<sst xmlns="'.self::NS.'"><si><t>a</t></si><si><t>b</t></si></sst>',
            'xl/styles.xml' => '<styleSheet xmlns="'.self::NS."\"><cellXfs>{$formats}</cellXfs></styleSheet>",
            ...$parts,
        ];

        $path = "{$dir}/book.xlsx";
        $zip = new \ZipArchive();
        $zip->open($path, \ZipArchive::CREATE | \ZipArchive::OVERWRITE) === true || throw new \RuntimeException("{$path} cannot be written");
        foreach ($entries as $name => $data) {
            $zip->addFromString($name, $data);
        }
        $zip->close();

        return $path;
    }

    private static function rel(string $id, string $type, string $target): string
    {
        return "<Relationship Id=\"{$id}\" Type=\"{$type}\" Target=\"{$target}\"/>";
    }

    private static function rels(string $body): string
    {
        return "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{$body}</Relationships>";
    }
}
