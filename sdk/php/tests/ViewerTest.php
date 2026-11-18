<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Grid;
use XlsxGraft\GridCell;
use XlsxGraft\SheetInfo;
use XlsxGraft\SheetKind;
use XlsxGraft\SheetState;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Tree;
use XlsxGraft\TreeRelationship;
use XlsxGraft\Viewer;

/**
 * The Viewer against the real CLI: what a workbook and the values given make
 * of a result (docs/sdk.md § Views in an SDK).
 */
final class ViewerTest extends TestCase
{
    protected function setUp(): void
    {
        Engine::cli();
    }

    public function testTheTreeOfAWorkbookListsItsPartsAndTheRelationshipsOfThePackage(): void
    {
        $result = (new Viewer())->tree(Engine::fixture('table.xlsx'));
        $tree = $result->value;

        self::assertTrue($result->success);
        self::assertNull($result->error);
        self::assertInstanceOf(Tree::class, $tree);
        self::assertTrue($tree->complete);
        self::assertContains('/xl/workbook.xml', array_map(static fn($part): string => $part->name, $tree->parts));
        self::assertContains('/xl/workbook.xml', array_map(static fn(TreeRelationship $r): ?string => $r->target->part, $tree->relationships));
    }

    public function testTheTreeOfTheBytesIsTheTreeOfTheFile(): void
    {
        $viewer = new Viewer();
        $path = Engine::fixture('table.xlsx');

        self::assertEquals($viewer->tree($path), $viewer->treeBytes((string) file_get_contents($path)));
    }

    public function testAnExternalRelationshipHasNoPart(): void
    {
        $tree = (new Viewer())->tree(Engine::fixture('external-link.xlsx'))->value;
        self::assertNotNull($tree);
        $relationships = $tree->relationships;
        foreach ($tree->parts as $part) {
            $relationships = [...$relationships, ...$part->relationships];
        }
        $external = array_values(array_filter($relationships, static fn(TreeRelationship $r): bool => $r->target->external !== null));

        self::assertNotSame([], $external);
        self::assertNull($external[0]->target->part);
    }

    public function testAFileThatIsNotThereIsIoAndItsTreeHasNoParts(): void
    {
        $dir = Engine::tempDir();
        $path = "{$dir}/missing.xlsx";
        try {
            $result = (new Viewer())->tree($path);

            self::assertTrue($result->failed());
            self::assertSame('io', $result->error?->code);
            self::assertStringContainsString($path, $result->error->message);
            self::assertSame([], $result->value?->parts);
        } finally {
            rmdir($dir);
        }
    }

    public function testBytesThatAreNoWorkbookAreADiagnosticAndATreeWithNoParts(): void
    {
        $result = (new Viewer())->treeBytes('not a zip');

        self::assertTrue($result->failed());
        self::assertNull($result->error);
        self::assertNotSame([], $result->diagnostics);
        self::assertSame([], $result->value?->parts);
    }

    public function testTheGridOfASheetHoldsItsCellsAndMerges(): void
    {
        $result = (new Viewer())->describeSheet(Engine::fixture('merged.xlsx'));
        $grid = $result->value;

        self::assertTrue($result->success);
        self::assertNull($result->error);
        self::assertInstanceOf(Grid::class, $grid);
        self::assertEquals(new SheetInfo(0, 'Sheet1', SheetKind::Worksheet, SheetState::Visible), $grid->sheet);
        self::assertSame(['A1:E8', 'A1:E8', 'A1:E8'], [$grid->dimension, $grid->used, $grid->shown]);
        self::assertSame([false, false, false], [$grid->truncated->rows, $grid->truncated->columns, $grid->truncated->merges]);
        self::assertTrue($grid->complete);
        self::assertSame(['A1:C1', 'A3:A5'], $grid->merges);
        self::assertSame(1, $grid->rows[0]->row);
        self::assertEquals(new GridCell('A1', 'string', 'Quarterly report', false), $grid->rows[0]->cells[0]);
        self::assertEquals(new GridCell('C3', 'number', 1.5, false), self::cell($grid, 'C3'));
        self::assertSame(2.0, self::cell($grid, 'C4')->value);
        self::assertEquals(new GridCell('C6', 'date', '2024-01-15', false), self::cell($grid, 'C6'));
    }

    public function testTheRangeAndTheRowLimitNarrowWhatIsShown(): void
    {
        $viewer = new Viewer();
        $path = Engine::fixture('merged.xlsx');

        self::assertSame('A1:C3', $viewer->describeSheet($path, range: 'A1:C3')->value?->shown);
        $rows = $viewer->describeSheet($path, maxRows: 2)->value;
        self::assertSame('A1:E2', $rows?->shown);
        self::assertTrue($rows->truncated->rows);
    }

    public function testASheetIsNamedByItsNameOrByItsIndex(): void
    {
        $viewer = new Viewer();
        $path = Engine::fixture('merged.xlsx');

        self::assertEquals($viewer->describeSheet($path), $viewer->describeSheet($path, sheet: 0));
        self::assertEquals($viewer->describeSheet($path), $viewer->describeSheet($path, sheet: 'Sheet1'));
    }

    public function testTheGridOfTheBytesIsTheGridOfTheFile(): void
    {
        $viewer = new Viewer();
        $path = Engine::fixture('merged.xlsx');
        $data = (string) file_get_contents($path);

        self::assertEquals($viewer->describeSheet($path, range: 'A1:C3'), $viewer->describeSheetBytes($data, range: 'A1:C3'));
    }

    public function testASheetThatIsNotFoundIsAnErrorAndAGridWithNoSheet(): void
    {
        $result = (new Viewer())->describeSheet(Engine::fixture('table.xlsx'), sheet: 'Nope');

        self::assertTrue($result->failed());
        self::assertSame('sheet_not_found', $result->error?->code);
        self::assertInstanceOf(Grid::class, $result->value);
        self::assertNull($result->value->sheet);
    }

    public function testAValueTheCliRefusesIsAnInvalidArgumentExceptionWithTheMessageOfTheCli(): void
    {
        $viewer = new Viewer();
        $path = Engine::fixture('table.xlsx');
        $refused = [
            '--max-rows' => static fn() => $viewer->describeSheet($path, maxRows: 0),
            '--max-columns' => static fn() => $viewer->describeSheetBytes('x', maxColumns: 0),
            '--range' => static fn() => $viewer->describeSheet($path, range: 'nope'),
            '--sheet-index' => static fn() => $viewer->describeSheet($path, sheet: -1),
            'invalid UTF-8' => static fn() => $viewer->describeSheet(Engine::fixture('merged.xlsx'), range: "\xFF"),
        ];
        foreach ($refused as $option => $call) {
            try {
                $call();
                self::fail("{$option} was not refused");
            } catch (\InvalidArgumentException $e) {
                self::assertStringContainsString($option, $e->getMessage());
                self::assertStringStartsNotWith('xlsx-graft: ', $e->getMessage());
            }
        }
    }

    private static function cell(Grid $grid, string $ref): GridCell
    {
        foreach ($grid->rows as $row) {
            foreach ($row->cells as $cell) {
                if ($cell->ref === $ref) {
                    return $cell;
                }
            }
        }
        self::fail("no cell {$ref}");
    }
}
