/**
 * Reads the keys of the `tree` and `describe_sheet` JSON, after `ok`, `error`
 * and `diagnostics`, into a {@link Tree} or a {@link Grid}. A key it does not
 * know is ignored, and the type of a value is not checked: JSON that lacks a
 * key, or whose value has the wrong type for what is done with it, throws a
 * TypeError.
 */

import * as engineDiagnostics from './engineDiagnostics.js';
import { Grid, GridCell, GridRow, GridTruncated } from './grid.js';
import { Tree, TreePart, TreeRelationship, TreeTarget } from './tree.js';
import { SheetInfo, sheetKind, sheetState } from './values.js';

type Record_ = Record<string, unknown>;

/** The tree the parsed JSON `view` holds. */
export function tree(view: Record_): Tree {
  return new Tree(
    get(view, 'complete'),
    get(view, 'relationshipsKnown'),
    relationships(get(view, 'relationships')),
    list(view, 'parts').map(part),
  );
}

/** The grid the parsed JSON `view` holds. */
export function grid(view: Record_): Grid {
  return new Grid(
    sheet(get(view, 'sheet')),
    get(view, 'dimension'),
    get(view, 'used'),
    get(view, 'shown'),
    truncated(engineDiagnostics.object(get(view, 'truncated'))),
    get(view, 'complete'),
    list(view, 'rows').map(row),
    get(view, 'merges'),
  );
}

// The value of `key`, taken to be of the type the caller needs.
function get<T>(record: Record_, key: string): T {
  return engineDiagnostics.field(record, key) as T;
}

function list(record: Record_, key: string): Record_[] {
  return get<Record_[]>(record, key);
}

function part(record: Record_): TreePart {
  return new TreePart(
    get(record, 'name'),
    get(record, 'contentType'),
    get(record, 'contentTypeKnown'),
    get(record, 'size'),
    get(record, 'compressedSize'),
    get(record, 'readable'),
    get(record, 'referenced'),
    get(record, 'relationshipsKnown'),
    relationships(get(record, 'relationships')),
  );
}

function relationships(records: Record_[]): TreeRelationship[] {
  return records.map((record) => {
    const target = engineDiagnostics.object(get(record, 'target'));
    return new TreeRelationship(
      get(record, 'id'),
      get(record, 'type'),
      new TreeTarget(
        Object.hasOwn(target, 'part') ? (target.part as string) : null,
        Object.hasOwn(target, 'external') ? (target.external as string) : null,
      ),
    );
  });
}

function sheet(value: Record_ | null): SheetInfo | null {
  if (value === null) {
    return null;
  }
  return new SheetInfo(
    get(value, 'index'),
    get(value, 'name'),
    sheetKind(get(value, 'kind')),
    sheetState(get(value, 'state')),
  );
}

function truncated(record: Record_): GridTruncated {
  return new GridTruncated(get(record, 'rows'), get(record, 'columns'), get(record, 'merges'));
}

function row(record: Record_): GridRow {
  return new GridRow(get(record, 'row'), list(record, 'cells').map(cell));
}

function cell(record: Record_): GridCell {
  return new GridCell(
    get(record, 'ref'),
    get(record, 'type'),
    get(record, 'value'),
    Object.hasOwn(record, 'truncated') ? (record.truncated as boolean) : false,
  );
}
