<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\CliJson;
use XlsxGraft\Grid;
use XlsxGraft\GridCell;
use XlsxGraft\GridRow;
use XlsxGraft\GridTruncated;
use XlsxGraft\SheetInfo;
use XlsxGraft\SheetKind;
use XlsxGraft\SheetState;
use XlsxGraft\Tree;
use XlsxGraft\TreePart;
use XlsxGraft\TreeRelationship;
use XlsxGraft\TreeTarget;
use XlsxGraft\ViewContent;

/**
 * How the JSON of `tree` and `describe-sheet` is read into the eight types:
 * every attribute of every type, from JSON whose strings and numbers all
 * differ, so that reading a key into another attribute changes a value. The
 * booleans take true and false, and the nullable attributes null, across the
 * items of a type.
 */
final class ViewContentTest extends TestCase
{
    private const REL_PART = '{"id":"rId1","type":"type-1","target":{"part":"/target-1"},"extra":1}';

    private const REL_EXTERNAL = '{"id":"rId2","type":"type-2","target":{"external":"external-2"}}';

    private const REL_NULLS = '{"id":"rId3","type":"type-3","target":{"part":null,"external":"external-3"}}';

    private const PART_A = '{"name":"/part-a","contentType":"type/a","contentTypeKnown":true,"size":120,"compressedSize":80,'
        .'"readable":false,"referenced":true,"relationshipsKnown":false,"relationships":['.self::REL_EXTERNAL.'],"extra":[]}';

    private const PART_B = '{"name":"/part-b","contentType":null,"contentTypeKnown":false,"size":13,"compressedSize":5,'
        .'"readable":true,"referenced":null,"relationshipsKnown":true,"relationships":['.self::REL_NULLS.']}';

    private const PART_C = '{"name":"/part-c","contentType":"type/c","contentTypeKnown":false,"size":17,"compressedSize":9,'
        .'"readable":true,"referenced":false,"relationshipsKnown":true,"relationships":[]}';

    public function testEveryKeyOfATreeIsReadIntoItsOwnAttribute(): void
    {
        $first = ViewContent::tree(CliJson::object('{"ok":true,"complete":true,"relationshipsKnown":false,'
            .'"relationships":['.self::REL_PART.','.self::REL_NULLS.'],"parts":['.self::PART_A.','.self::PART_B.'],"extra":null}'));
        $second = ViewContent::tree(CliJson::object('{"complete":false,"relationshipsKnown":true,'
            .'"relationships":[],"parts":['.self::PART_C.']}'));

        self::assertSameValue(new Tree(true, false, [
            new TreeRelationship('rId1', 'type-1', new TreeTarget('/target-1', null)),
            new TreeRelationship('rId3', 'type-3', new TreeTarget(null, 'external-3')),
        ], [
            new TreePart('/part-a', 'type/a', true, 120, 80, false, true, false, [
                new TreeRelationship('rId2', 'type-2', new TreeTarget(null, 'external-2')),
            ]),
            new TreePart('/part-b', null, false, 13, 5, true, null, true, [
                new TreeRelationship('rId3', 'type-3', new TreeTarget(null, 'external-3')),
            ]),
        ]), $first);
        self::assertSameValue(new Tree(false, true, [], [
            new TreePart('/part-c', 'type/c', false, 17, 9, true, false, true, []),
        ]), $second);
    }

    public function testEveryKeyOfAGridIsReadIntoItsOwnAttribute(): void
    {
        $first = ViewContent::grid(CliJson::object('{"ok":true,"sheet":{"index":2,"name":"Data-1","kind":"chartsheet","state":"veryHidden","extra":0},'
            .'"dimension":"A1:D9","used":"B2:C7","shown":"B2:C4","truncated":{"rows":true,"columns":false,"merges":true},"complete":false,'
            .'"rows":[{"row":2,"cells":['
            .'{"ref":"B2","type":"string","value":"text-1","truncated":true},'
            .'{"ref":"C2","type":"number","value":90000},'
            .'{"ref":"D2","type":"number","value":2.0,"truncated":false}]},'
            .'{"row":5,"cells":['
            .'{"ref":"B5","type":"number","value":1.5},'
            .'{"ref":"C5","type":"boolean","value":true},'
            .'{"ref":"D5","type":"boolean","value":false},'
            .'{"ref":"E5","type":"date","value":"2024-01-15","extra":1}]}],'
            .'"merges":["B2:C2","B4:B5"],"extra":{}}'));
        $second = ViewContent::grid(CliJson::object('{"sheet":null,"dimension":null,"used":null,"shown":null,'
            .'"truncated":{"rows":false,"columns":true,"merges":false},"complete":true,"rows":[],"merges":[]}'));
        $unknown = ViewContent::grid(CliJson::object('{"sheet":{"index":0,"name":"Other","kind":"future","state":"later"},'
            .'"dimension":"A1","used":"A1","shown":"A1","truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"rows":[],"merges":[]}'));

        self::assertSameValue(new Grid(
            new SheetInfo(2, 'Data-1', SheetKind::Chartsheet, SheetState::VeryHidden),
            'A1:D9',
            'B2:C7',
            'B2:C4',
            new GridTruncated(true, false, true),
            false,
            [
                new GridRow(2, [
                    new GridCell('B2', 'string', 'text-1', true),
                    new GridCell('C2', 'number', 90000, false),
                    new GridCell('D2', 'number', 2.0, false),
                ]),
                new GridRow(5, [
                    new GridCell('B5', 'number', 1.5, false),
                    new GridCell('C5', 'boolean', true, false),
                    new GridCell('D5', 'boolean', false, false),
                    new GridCell('E5', 'date', '2024-01-15', false),
                ]),
            ],
            ['B2:C2', 'B4:B5'],
        ), $first);
        self::assertSameValue(new Grid(null, null, null, null, new GridTruncated(false, true, false), true, [], []), $second);

        $numbers = ViewContent::grid(CliJson::object('{"sheet":null,"dimension":null,"used":null,"shown":null,'
            .'"truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"rows":[{"row":1,"cells":['
            .'{"ref":"A1","type":"number","value":-3},{"ref":"B1","type":"number","value":1e3},'
            .'{"ref":"C1","type":"number","value":12345678901234567890},{"ref":"D1","type":"number","value":1e999}]}],"merges":[]}'));
        self::assertSameValue([-3, 1000.0, 1.2345678901234567E+19, INF], array_map(static fn(GridCell $c) => $c->value, $numbers->rows[0]->cells));
        self::assertSameValue(new SheetInfo(0, 'Other', SheetKind::Unknown, SheetState::Unknown), $unknown->sheet);
    }

    public function testAKeyThatIsMissingOrOfAnotherTypeIsUnreadable(): void
    {
        $cell = '{"ref":"A1","type":"string","value":"v"}';
        $grid = static fn(string $merges, string $cell): string => '{"sheet":null,"dimension":null,"used":null,"shown":null,'
            .'"truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"rows":[{"row":1,"cells":['.$cell.']}],"merges":'.$merges.'}';
        $unreadable = [
            'no complete' => static fn() => ViewContent::tree(CliJson::object('{"relationshipsKnown":true,"relationships":[],"parts":[]}')),
            'a part that is not an object' => static fn() => ViewContent::tree(CliJson::object('{"complete":true,"relationshipsKnown":true,"relationships":[],"parts":[1]}')),
            'a size that is a string' => static fn() => ViewContent::tree(CliJson::object('{"complete":true,"relationshipsKnown":true,"relationships":[],"parts":['
                .str_replace('"size":13', '"size":"13"', self::PART_B).']}')),
            'a target that is null' => static fn() => ViewContent::tree(CliJson::object('{"complete":true,"relationshipsKnown":true,"relationships":['
                .'{"id":"a","type":"b","target":null}],"parts":[]}')),
            'a part name that is not a string' => static fn() => ViewContent::tree(CliJson::object('{"complete":true,"relationshipsKnown":true,"relationships":['
                .'{"id":"a","type":"b","target":{"part":1}}],"parts":[]}')),
            'merges that are not strings' => static fn() => ViewContent::grid(CliJson::object($grid('[1]', $cell))),
            'merges that are no array' => static fn() => ViewContent::grid(CliJson::object($grid('"A1:B2"', $cell))),
            'a cell value that is null' => static fn() => ViewContent::grid(CliJson::object($grid('[]', '{"ref":"A1","type":"string","value":null}'))),
            'a cell value that is an array' => static fn() => ViewContent::grid(CliJson::object($grid('[]', '{"ref":"A1","type":"string","value":[]}'))),
            'a cell truncated that is a string' => static fn() => ViewContent::grid(CliJson::object($grid('[]', '{"ref":"A1","type":"string","value":"v","truncated":"yes"}'))),
            'no truncated' => static fn() => ViewContent::grid(CliJson::object('{"sheet":null,"dimension":null,"used":null,"shown":null,"complete":true,"rows":[],"merges":[]}')),
        ];
        foreach ($unreadable as $name => $read) {
            try {
                $read();
                self::fail("{$name} was read");
            } catch (\UnexpectedValueException) {
                $this->addToAssertionCount(1);
            }
        }
    }

    /** The values and their types: a float 2.0 is not the int 2, which `assertEquals` would not tell. */
    private static function assertSameValue(mixed $expected, mixed $actual): void
    {
        self::assertSame(var_export($expected, true), var_export($actual, true));
    }
}
