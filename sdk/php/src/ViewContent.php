<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Reads the keys of a view's JSON after `ok`, `error` and `diagnostics` into a
 * {@see Tree} or a {@see Grid}. A key it does not know is skipped. A key that is
 * missing or of another type throws an \UnexpectedValueException, which
 * {@see ViewReport} turns into `invalid_output`; nothing here checks a value
 * beyond the type of the attribute it is read into.
 *
 * @internal
 */
final class ViewContent
{
    /** The Tree that the object `xlsx-graft tree` printed holds. */
    public static function tree(\stdClass $view): Tree
    {
        return new Tree(
            CliJson::bool($view, 'complete'),
            CliJson::bool($view, 'relationshipsKnown'),
            self::relationships($view),
            array_map(self::part(...), CliJson::objects($view, 'parts')),
        );
    }

    /** The Grid that the object `xlsx-graft describe-sheet` printed holds. */
    public static function grid(\stdClass $view): Grid
    {
        $truncated = CliJson::child($view, 'truncated');

        return new Grid(
            self::sheet(CliJson::nullableChild($view, 'sheet')),
            CliJson::nullableString($view, 'dimension'),
            CliJson::nullableString($view, 'used'),
            CliJson::nullableString($view, 'shown'),
            new GridTruncated(
                CliJson::bool($truncated, 'rows'),
                CliJson::bool($truncated, 'columns'),
                CliJson::bool($truncated, 'merges'),
            ),
            CliJson::bool($view, 'complete'),
            array_map(self::row(...), CliJson::objects($view, 'rows')),
            CliJson::strings($view, 'merges'),
        );
    }

    private static function part(\stdClass $part): TreePart
    {
        return new TreePart(
            CliJson::string($part, 'name'),
            CliJson::nullableString($part, 'contentType'),
            CliJson::bool($part, 'contentTypeKnown'),
            CliJson::int($part, 'size'),
            CliJson::int($part, 'compressedSize'),
            CliJson::bool($part, 'readable'),
            CliJson::nullableBool($part, 'referenced'),
            CliJson::bool($part, 'relationshipsKnown'),
            self::relationships($part),
        );
    }

    /** @return list<TreeRelationship> */
    private static function relationships(\stdClass $owner): array
    {
        return array_map(static function (\stdClass $relationship): TreeRelationship {
            $target = CliJson::child($relationship, 'target');

            // `part` or `external` is absent when the target is not that kind.
            return new TreeRelationship(
                CliJson::string($relationship, 'id'),
                CliJson::string($relationship, 'type'),
                new TreeTarget(
                    property_exists($target, 'part') ? CliJson::nullableString($target, 'part') : null,
                    property_exists($target, 'external') ? CliJson::nullableString($target, 'external') : null,
                ),
            );
        }, CliJson::objects($owner, 'relationships'));
    }

    // The same four keys as BookCore reads for the sheets of a book.
    private static function sheet(?\stdClass $sheet): ?SheetInfo
    {
        return $sheet === null ? null : new SheetInfo(
            CliJson::int($sheet, 'index'),
            CliJson::string($sheet, 'name'),
            SheetKind::tryFrom(CliJson::string($sheet, 'kind')) ?? SheetKind::Unknown,
            SheetState::tryFrom(CliJson::string($sheet, 'state')) ?? SheetState::Unknown,
        );
    }

    private static function row(\stdClass $row): GridRow
    {
        return new GridRow(
            CliJson::int($row, 'row'),
            array_map(static fn(\stdClass $cell): GridCell => new GridCell(
                CliJson::string($cell, 'ref'),
                CliJson::string($cell, 'type'),
                CliJson::scalar($cell, 'value'),
                property_exists($cell, 'truncated') && CliJson::bool($cell, 'truncated'),
            ), CliJson::objects($row, 'cells')),
        );
    }
}
