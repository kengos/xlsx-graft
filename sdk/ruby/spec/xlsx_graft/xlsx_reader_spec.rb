# frozen_string_literal: true

require "pathname"

RSpec.describe XlsxGraft::XlsxReader do
  let(:bytes) do
    Workbook.xlsx(
      Workbook.worksheet("Data", <<~XML.delete("\n")),
        <row r="1"><c r="A1" t="s"><v>1</v></c><c r="C1"><v>3</v></c><c r="D1" t="b"><v>1</v></c></row>
        <row r="3"><c r="B3" s="1"><v>45322</v></c><c r="C3" s="2"><v>0.75</v></c>
        <c r="D3" s="3"><v>45322.000005787037</v></c><c r="E3" s="4"><v>1.5</v></c>
        <c r="F3" t="e"><v>#N/A</v></c><c r="G3" t="inlineStr"><is><t></t></is></c></row>
        <row r="4"/>
      XML
      Workbook.chartsheet("Chart"),
      Workbook.worksheet("Hidden", "", state: "veryHidden")
    )
  end

  def rows_of(bytes, which = 0, **)
    rows = []
    result = described_class.open_bytes(bytes, **) do |book|
      book.sheet(which).each_row { |row| rows << row }
    end
    [result, rows]
  end

  describe ".open_bytes" do
    it "yields each row with its number and its values from column A" do
      result, rows = rows_of(bytes, "data")
      expect(result).to have_attributes(success?: true, error: nil, diagnostics: [])
      expect(rows.map(&:number)).to eq([1, 3, 4])
      expect(rows[0].values).to eq(["b", nil, 3.0, true])
      expect(rows[1].values).to eq(
        [nil, Date.new(2024, 1, 31), XlsxGraft::TimeOfDay.new(18, 0, 0, 0),
         Time.utc(2024, 1, 31, 0, 0, Rational(1, 2)), 36 * 3600.0,
         XlsxGraft::CellError.new("#N/A"), ""]
      )
      expect(rows[2].values).to eq([])
    end

    it "reads a date-time in the zone asked for" do
      _, rows = rows_of(bytes, timezone: "+09:00")
      expect(rows[1].values[3]).to have_attributes(utc_offset: 9 * 3600, hour: 0, usec: 500_000)
    end

    it "stops with datetime_unrepresentable for a time the zone object cannot place" do
      zone = Object.new
      def zone.local_to_utc(time)
        raise ArgumentError, "skipped by daylight saving" if time.year == 2024

        time
      end

      result, rows = rows_of(bytes, timezone: zone)
      expect(result.error).to have_attributes(
        code: "datetime_unrepresentable",
        message: start_with("row 3: 2024-01-31 00:00:00.500 cannot")
      )
      expect(rows.map(&:number)).to eq([1])
    end

    it "returns the block's value and lists the sheets" do
      result = described_class.open_bytes(bytes, &:sheets)
      expect(result.value.map(&:to_h)).to eq(
        [{ index: 0, name: "Data", kind: "worksheet", state: "visible" },
         { index: 1, name: "Chart", kind: "chartsheet", state: "visible" },
         { index: 2, name: "Hidden", kind: "worksheet", state: "veryHidden" }]
      )
    end

    it "gives the rows as an Enumerator without a block" do
      result = described_class.open_bytes(bytes) { |book| book.sheet.each_row.map(&:number) }
      expect(result.value).to eq([1, 3, 4])
    end

    it "reads a sheet again from its start" do
      result = described_class.open_bytes(bytes) do |book|
        sheet = book.sheet("Data")
        sheet.each_row.first(1) + sheet.each_row.to_a
      end
      expect(result.value.map(&:number)).to eq([1, 1, 3, 4])
    end
  end

  describe "a sheet refused" do
    {
      "Missing" => ["sheet_not_found", 'no sheet is named "Missing"'],
      9 => ["sheet_not_found", "there is no sheet at index 9: the workbook has 3, from index 0"],
      "Chart" => ["sheet_not_worksheet", 'sheet "Chart" is a chartsheet, which holds no cells'],
      1 => ["sheet_not_worksheet", 'sheet "Chart" is a chartsheet, which holds no cells']
    }.each do |which, (code, message)|
      it "fails with #{code} for #{which.inspect}, reading no rows" do
        result, rows = rows_of(bytes, which)
        error = have_attributes(code:, severity: "error", message:)
        expect(result).to have_attributes(success?: false, value: nil, error:)
        expect(rows).to eq([])
      end
    end

    it "goes on with the block, and keeps the refusal as the error" do
      result = described_class.open_bytes(bytes) do |book|
        book.sheet("Missing").each_row { nil }
        book.sheet("Data").each_row.count
      end
      expect(result).to have_attributes(success?: false, value: nil,
                                        error: have_attributes(code: "sheet_not_found"))
    end

    it "fails with sheet_unreadable when the sheet's part is not known" do
      lost = Workbook.xlsx(Workbook.unrelated("Lost"))
      result = described_class.open_bytes(lost) { |book| book.sheet("Lost").each_row { nil } }
      expect(result.error).to have_attributes(
        code: "sheet_unreadable", message: 'sheet "Lost" cannot be read; the diagnostics say why'
      )
      expect(result.diagnostics).not_to be_empty
    end

    it "fails with sheet_ambiguous when two sheets have the name" do
      twice = Workbook.xlsx(Workbook.worksheet("Data"), Workbook.worksheet("DATA"))
      result, = rows_of(twice, "data")
      expect(result.error).to have_attributes(
        code: "sheet_ambiguous", message: 'more than one sheet is named "data", letter case aside'
      )
      expect(result.diagnostics.map(&:code)).to include("sheet_name_duplicate")
    end

    it "refuses a sheet name that is not UTF-8" do
      result, = rows_of(bytes, "\xFF".b)
      expect(result.error.code).to eq("invalid_arg")
    end

    it "fails with workbook_unreadable, listing no sheets, when the bytes are no workbook" do
      result, = rows_of("not a zip")
      sheets = described_class.open_bytes("not a zip", &:sheets)
      expect(result).to have_attributes(success?: false, error: have_attributes(
        code: "workbook_unreadable", message: start_with("the workbook cannot be read")
      ))
      expect(sheets.diagnostics).not_to be_empty
    end
  end

  it "fails without an error when a problem stops the reading" do
    twice = '<row r="1"><c r="A1"><v>1</v></c></row><row r="1"/>'
    stopped = Workbook.xlsx(Workbook.worksheet("Data", twice))
    result, rows = rows_of(stopped)
    expect(rows.map(&:number)).to eq([1])
    expect(result).to have_attributes(success?: false, value: nil, error: nil)
    expect(result.diagnostics.first).to have_attributes(
      code: "cell_order_invalid", severity: "error",
      location: include("part" => end_with("sheet1.xml"))
    )
  end

  it "closes the book when the block raises, and raises on" do
    kept = nil
    expect do
      described_class.open_bytes(bytes) do |book|
        kept = book
        raise "boom"
      end
    end.to raise_error(RuntimeError, "boom")
    expect { kept.sheets }.to raise_error(XlsxGraft::ClosedError, /closed/)
    expect { kept.sheet("Data") }.to raise_error(XlsxGraft::ClosedError)
  end

  it "raises ClosedError for a sheet read after the block" do
    sheet = described_class.open_bytes(bytes, &:sheet).value
    expect { sheet.each_row { nil } }.to raise_error(XlsxGraft::ClosedError)
    expect { sheet.each_row.next }.to raise_error(XlsxGraft::ClosedError)
  end

  it "closes the rows an Enumerator left open" do
    result = described_class.open_bytes(bytes) do |book|
      rows = book.sheet.each_row
      rows.next.number
    end
    expect(result).to have_attributes(success?: true, value: 1)
  end

  it "raises ArgumentError without a block, for a zone Time does not take, and for a bad sheet" do
    expect { described_class.open_bytes(bytes) }.to raise_error(ArgumentError, /block is required/)
    expect { described_class.open_bytes(bytes, timezone: "Mars") { nil } }
      .to raise_error(ArgumentError)
    expect { described_class.open_bytes(bytes, timezone: Object.new) { nil } }
      .to raise_error(ArgumentError, /no zone Time takes/)
    [:data, -1, 2**64].each do |which|
      expect { described_class.open_bytes(bytes) { |book| book.sheet(which) } }
        .to raise_error(ArgumentError, /a name or an index from 0, not #{which.inspect}/)
    end
  end

  describe ".open" do
    it "reads the workbook at a path, a Pathname too" do
      Workbook.write(bytes) do |path|
        result = described_class.open(Pathname(path)) { |book| book.sheet.each_row.count }
        expect(result.value).to eq(3)
      end
    end

    it "fails with io, the host's reason, when the file cannot be opened" do
      result = described_class.open("/nowhere/book.xlsx") { nil }
      expect(result).to have_attributes(
        success?: false, diagnostics: [],
        error: have_attributes(code: "io", message: include("nowhere"))
      )
    end

    it "fails with invalid_arg for a path that is not UTF-8" do
      result = described_class.open("/tmp/\xFF.xlsx".b) { nil }
      expect(result.error.code).to eq("invalid_arg")
    end

    it "passes library: on to Library.open" do
      allow(XlsxGraft::Library).to receive(:open).and_call_original
      described_class.open("/nowhere.xlsx", library: "/elsewhere.so") { nil }
      expect(XlsxGraft::Library).to have_received(:open).with(path: "/elsewhere.so")
    end
  end
end
