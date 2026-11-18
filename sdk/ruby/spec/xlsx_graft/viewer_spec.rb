# frozen_string_literal: true

RSpec.describe XlsxGraft::Viewer do
  let(:viewer) { described_class.new }
  let(:table) { File.join(FIXTURES, "table.xlsx") }
  let(:merged) { File.join(FIXTURES, "merged.xlsx") }

  describe "#tree" do
    it "lists the parts and the relationships of the workbook at a path" do
      result = viewer.tree(table)
      tree = result.value
      expect(result).to have_attributes(success?: true, error: nil)
      expect(tree).to be_a(XlsxGraft::Tree).and have_attributes(complete: true)
      expect(tree.parts.map(&:name)).to include("/xl/workbook.xml")
      expect(tree.relationships.map(&:target).map(&:part)).to include("/xl/workbook.xml")
    end

    it "gives the same Tree for a Pathname and for the bytes of the file" do
      tree = viewer.tree(table).value
      expect(viewer.tree(Pathname(table)).value).to eq(tree)
      expect(viewer.tree_bytes(File.binread(table)).value).to eq(tree)
    end

    it "gives a target outside the package as external, with no part" do
      tree = viewer.tree(File.join(FIXTURES, "external-link.xlsx")).value
      targets = tree.parts.flat_map { |part| part.relationships.map(&:target) }
      external = targets.find(&:external)
      expect(external).to have_attributes(part: nil, external: be_a(String))
    end

    it "fails with an io error and a Tree with no parts for a file that does not exist" do
      result = viewer.tree("/nowhere/book.xlsx")
      expect(result).to have_attributes(success?: false)
      expect(result.error).to have_attributes(code: "io")
      expect(result.value).to be_a(XlsxGraft::Tree).and have_attributes(parts: [])
    end

    it "fails with diagnostics and a Tree with no parts for bytes that are not a workbook" do
      result = viewer.tree_bytes("not a zip")
      expect(result).to have_attributes(success?: false, error: nil)
      expect(result.diagnostics).not_to be_empty
      expect(result.value).to be_a(XlsxGraft::Tree).and have_attributes(parts: [])
    end

    it "reads a file whose name is not UTF-8, and has no value when it cannot be read" do
      Dir.mktmpdir do |dir|
        name = File.join(dir.b, "\xFF.xlsx".b)
        FileUtils.cp(table, name)
        expect(viewer.tree(name).value).to eq(viewer.tree(table).value)
        missing = viewer.tree(File.join(dir.b, "\xFE.xlsx".b))
        expect(missing.error).to have_attributes(code: "io", message: include("could not be read"))
        expect(missing.value).to be_nil
      end
    end
  end

  describe "#describe_sheet" do
    let(:first_row) do
      XlsxGraft::GridRow.new(
        row: 1, cells: [XlsxGraft::GridCell.new(ref: "A1", type: "string",
                                                value: "Quarterly report", truncated: false)]
      )
    end

    def cell(grid, ref)
      grid.rows.flat_map(&:cells).find { |c| c.ref == ref }
    end

    it "shows the sheet, the ranges and the merges of the first sheet" do
      result = viewer.describe_sheet(merged)
      expect(result).to have_attributes(success?: true, error: nil)
      expect(result.value).to have_attributes(
        sheet: XlsxGraft::SheetInfo.new(index: 0, name: "Sheet1", kind: "worksheet",
                                        state: "visible"),
        dimension: "A1:E8", used: "A1:E8", shown: "A1:E8", complete: true,
        truncated: XlsxGraft::GridTruncated.new(rows: false, columns: false, merges: false),
        merges: ["A1:C1", "A3:A5"]
      )
    end

    it "shows the cells of the first sheet" do
      grid = viewer.describe_sheet(merged).value
      expect(grid.rows.first).to eq(first_row)
      expect(cell(grid, "C3")).to have_attributes(type: "number", value: 1.5)
      expect(cell(grid, "C4").value).to be_a(Float).and eq(2.0)
      expect(cell(grid, "C6")).to have_attributes(type: "date", value: "2024-01-15")
    end

    it "gives the same Grid for a Pathname and for the bytes of the file" do
      grid = viewer.describe_sheet(merged).value
      expect(viewer.describe_sheet(Pathname(merged)).value).to eq(grid)
      expect(viewer.describe_sheet_bytes(File.binread(merged)).value).to eq(grid)
    end

    it "takes the window and the sheet that the keywords name" do
      expect(viewer.describe_sheet(merged, range: "A1:C3").value.shown).to eq("A1:C3")
      expect(viewer.describe_sheet(merged, max_rows: 2).value.shown).to eq("A1:E2")
      expect(viewer.describe_sheet(merged, max_columns: 2).value.shown).to eq("A1:B8")
      expect(viewer.describe_sheet(merged, sheet: :Sheet1).value.sheet.name).to eq("Sheet1")
      bytes = viewer.describe_sheet_bytes(File.binread(merged), sheet: "Sheet1", max_rows: 2)
      expect(bytes.value.shown).to eq("A1:E2")
    end

    it "takes the index of the sheet as an Integer" do
      by_index = viewer.describe_sheet(merged, sheet: 0).value
      expect(by_index).to eq(viewer.describe_sheet(merged).value)
    end

    it "reads a file whose name is not UTF-8 as bytes and shows the same Grid" do
      Dir.mktmpdir do |dir|
        name = File.join(dir.b, "\xFF.xlsx".b)
        FileUtils.cp(merged, name)
        expect(viewer.describe_sheet(name).value).to eq(viewer.describe_sheet(merged).value)
      end
    end

    it "fails with sheet_not_found and a Grid whose sheet is nil" do
      result = viewer.describe_sheet(table, sheet: "Nope")
      expect(result).to have_attributes(success?: false)
      expect(result.error).to have_attributes(code: "sheet_not_found")
      expect(result.value).to be_a(XlsxGraft::Grid).and have_attributes(sheet: nil, rows: [])
    end

    it "raises ArgumentError with the engine's message for a value the engine refuses" do
      expect { viewer.describe_sheet(merged, range: "nope") }.to raise_error(ArgumentError, /range/)
      expect { viewer.describe_sheet(merged, max_rows: 0) }.to raise_error(ArgumentError, /maxRows/)
      expect { viewer.describe_sheet(merged, sheet: 1.5) }.to raise_error(ArgumentError, /sheet/)
    end

    it "raises ArgumentError before the call for options that cannot be sent" do
      expect { viewer.describe_sheet("/nowhere", max_rows: 10**400) }
        .to raise_error(ArgumentError, /max_rows/)
      expect { viewer.describe_sheet(merged, range: "\xFF".b) }.to raise_error(ArgumentError)
      expect { viewer.describe_sheet(merged, range: "A" * (2 << 20)) }
        .to raise_error(ArgumentError, /the engine takes 2097152/)
      expect { viewer.describe_sheet_bytes("", max_columns: Float::NAN) }
        .to raise_error(ArgumentError)
    end
  end

  it "may be shared by threads that call the viewer at once" do
    bytes = File.binread(merged)
    grids = Array.new(4) { Thread.new { viewer.describe_sheet_bytes(bytes).value } }.map(&:value)
    expect(grids.uniq.size).to eq(1)
    expect(grids.first.rows).not_to be_empty
  end
end
