/**
 * Workbooks for the tests to read, built as a test asks: a zip of stored
 * entries holding a package with a shared strings table of "a" and "b",
 * styles whose cell formats 1 to 4 are a date, a time, a date-time and an
 * elapsed time, and the sheets the test gives. The same workbook as
 * engine/capi/src/tests/workbook.rs builds, written again in TypeScript.
 */

import { crc32 } from 'node:zlib';

const NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main';
const TYPES = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const RELS = 'http://schemas.openxmlformats.org/package/2006/relationships';
const CONTENT_TYPES =
  '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">' +
  '<Default Extension="rels" ' +
  'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>' +
  '<Default Extension="xml" ContentType="application/xml"/></Types>';

/**
 * A sheet: its name, its `state` (null: none written), the type of its
 * relationship (null: its r:id names none) and its part's XML.
 */
export interface Sheet {
  name: string;
  state: string | null;
  relType: string | null;
  xml: string;
}

/** A worksheet named `name` whose sheetData holds `rows`. */
export function worksheet(name: string, rows = '', state: string | null = null): Sheet {
  const xml = `<worksheet xmlns="${NS}"><sheetData>${rows}</sheetData></worksheet>`;
  return { name, state, relType: `${TYPES}/worksheet`, xml };
}

export function chartsheet(name: string): Sheet {
  return { name, state: null, relType: `${TYPES}/chartsheet`, xml: `<chartsheet xmlns="${NS}"/>` };
}

/** A sheet named `name` whose r:id names no relationship. */
export function unrelated(name: string): Sheet {
  return { name, state: null, relType: null, xml: '' };
}

/** The bytes of a workbook of `sheets`, in order, each at xl/sheets/sheet<n>.xml. */
export function xlsx(...sheets: Sheet[]): Buffer {
  const numbered = sheets.map((sheet, i) => [i + 1, sheet] as const);
  const entries = new Map([
    ['[Content_Types].xml', CONTENT_TYPES],
    ['_rels/.rels', rels(rel('rId1', `${TYPES}/officeDocument`, 'xl/workbook.xml'))],
    ['xl/workbook.xml', workbook(numbered)],
    ['xl/_rels/workbook.xml.rels', rels(related(numbered))],
    ['xl/sst.xml', `<sst xmlns="${NS}"><si><t>a</t></si><si><t>b</t></si></sst>`],
    ['xl/styles.xml', styles()],
  ]);
  for (const [n, sheet] of numbered) {
    if (sheet.relType) {
      entries.set(`xl/sheets/sheet${n}.xml`, sheet.xml);
    }
  }
  return zip(entries);
}

type Numbered = readonly (readonly [number, Sheet])[];

function workbook(numbered: Numbered): string {
  const listed = numbered
    .map(
      ([n, sheet]) =>
        `<sheet name="${sheet.name}" sheetId="${n}"` +
        (sheet.state ? ` state="${sheet.state}"` : '') +
        ` r:id="rId${n}"/>`,
    )
    .join('');
  return `<workbook xmlns="${NS}" xmlns:r="${TYPES}"><sheets>${listed}</sheets></workbook>`;
}

function related(numbered: Numbered): string {
  const sheets = numbered
    .filter(([, sheet]) => sheet.relType)
    .map(([n, sheet]) => rel(`rId${n}`, sheet.relType as string, `sheets/sheet${n}.xml`))
    .join('');
  return (
    rel('rIdS', `${TYPES}/sharedStrings`, 'sst.xml') +
    rel('rIdT', `${TYPES}/styles`, 'styles.xml') +
    sheets
  );
}

function styles(): string {
  const formats = [0, 14, 21, 22, 46].map((id) => `<xf numFmtId="${id}"/>`).join('');
  return `<styleSheet xmlns="${NS}"><cellXfs>${formats}</cellXfs></styleSheet>`;
}

function rel(id: string, type: string, target: string): string {
  return `<Relationship Id="${id}" Type="${type}" Target="${target}"/>`;
}

function rels(body: string): string {
  return `<Relationships xmlns="${RELS}">${body}</Relationships>`;
}

/** The zip of `entries`, each stored, in order. */
function zip(entries: Map<string, string>): Buffer {
  const out: Buffer[] = [];
  const directory: Buffer[] = [];
  let offset = 0;
  for (const [name, text] of entries) {
    const data = Buffer.from(text);
    const path = Buffer.from(name);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc32(data), 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(path.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc32(data), 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(path.length, 28);
    central.writeUInt32LE(offset, 42);
    out.push(local, path, data);
    directory.push(central, path);
    offset += local.length + path.length + data.length;
  }
  const size = directory.reduce((sum, part) => sum + part.length, 0);
  const trailer = Buffer.alloc(22);
  trailer.writeUInt32LE(0x06054b50, 0);
  trailer.writeUInt16LE(entries.size, 8);
  trailer.writeUInt16LE(entries.size, 10);
  trailer.writeUInt32LE(size, 12);
  trailer.writeUInt32LE(offset, 16);
  return Buffer.concat([...out, ...directory, trailer]);
}
