# frozen_string_literal: true

# What the engine cannot be made to do from a workbook: panic, fail the host
# while lending the diagnostics, answer a status or lend a value newer than
# the gem. Each spec reads a real workbook through a Capi of which one call
# is replaced.
RSpec.describe XlsxGraft::Book do
  let(:bytes) { Workbook.xlsx(Workbook.worksheet("Data", '<row r="1"/><row r="2"/>')) }
  let(:capi) { XlsxGraft::Capi.new(XlsxGraft::Library.open) }

  before do
    allow(XlsxGraft::Capi).to receive(:new).and_return(capi)
  end

  # Reads the first sheet: the result and the rows' numbers.
  def read
    rows = []
    result = XlsxGraft::XlsxReader.open_bytes(bytes) do |book|
      book.sheet.each_row { |row| rows << row.number }
      :done
    end
    [result, rows]
  end

  context "when reading a row panics" do
    before do
      # The engine refuses every call after a panic.
      allow(capi).to receive_messages(rows_next: [2, 0, 0, 0], book_diagnostics: [2, ""])
    end

    it "fails with panic, reading no diagnostics the panic left" do
      result, rows = read
      expect(result).to have_attributes(success?: false, diagnostics: [],
                                        error: have_attributes(code: "panic"))
      expect(rows).to eq([])
    end
  end

  context "when the library answers a status the gem does not know" do
    before do
      allow(capi).to receive(:rows_open).and_return([42, 0])
    end

    it "fails with unknown_status" do
      result, = read
      expect(result.error).to have_attributes(code: "unknown_status", message: include("42"))
    end
  end

  context "when a value is of a kind newer than the gem" do
    before do
      allow(XlsxGraft::CellValues).to receive(:call).and_raise(
        XlsxGraft::CellValues::Refused.new("unknown_value", "the engine returned a value of kind 9")
      )
    end

    it "stops the sheet with unknown_value" do
      result, rows = read
      expect(result.error).to have_attributes(
        code: "unknown_value", message: "row 1: the engine returned a value of kind 9"
      )
      expect(rows).to eq([])
    end
  end

  context "when a stop of the gem's comes before a refused sheet" do
    before do
      allow(XlsxGraft::CellValues).to receive(:call).and_raise(
        XlsxGraft::CellValues::Refused.new("unknown_value", "kind 9")
      )
    end

    it "keeps the first as the error" do
      result = XlsxGraft::XlsxReader.open_bytes(bytes) do |book|
        book.sheet.each_row { nil }
        book.sheet("Missing").each_row { nil }
      end
      expect(result.error).to have_attributes(code: "unknown_value")
    end
  end

  context "when the diagnostics cannot be lent" do
    before do
      allow(capi).to receive_messages(book_diagnostics: [3, ""], error_message: "the disk is full")
    end

    it "fails with io, saying why" do
      result, rows = read
      expect(result.error).to have_attributes(code: "io", message: "the disk is full")
      expect(rows).to eq([1, 2])
    end
  end

  context "when the diagnostics are not the JSON the gem reads" do
    before do
      allow(capi).to receive(:book_diagnostics).and_return([0, "{}"])
    end

    it "fails with diagnostics_unreadable" do
      result, = read
      expect(result.error).to have_attributes(code: "diagnostics_unreadable",
                                              message: include("key not found"))
    end
  end

  context "when the caller closes the book in the block" do
    before do
      allow(capi).to receive(:book_close).and_call_original
    end

    it "closes it once" do
      result = XlsxGraft::XlsxReader.open_bytes(bytes) do |book|
        book.close
        book.close
      end
      expect(result.success?).to be(true)
      expect(capi).to have_received(:book_close).once
    end
  end

  context "when the block leaves an Enumerator's rows open" do
    before do
      allow(capi).to receive(:rows_close).and_call_original
    end

    it "frees them at the end" do
      result = XlsxGraft::XlsxReader.open_bytes(bytes) { |book| book.sheet.each_row.next.number }
      expect(result.value).to eq(1)
      expect(capi).to have_received(:rows_close).once
    end
  end

  context "when freeing a rows handle panics" do
    before do
      allow(capi).to receive(:rows_close).and_return(2)
    end

    it "fails with panic" do
      result, rows = read
      expect(result.error).to have_attributes(code: "panic")
      expect(rows).to eq([1, 2])
    end
  end

  context "when a sheet is of a kind or state newer than the gem" do
    before do
      allow(capi).to receive(:book_sheets).and_return([0, [["New", 7, 9]]])
    end

    it "lists it as unknown" do
      result = XlsxGraft::XlsxReader.open_bytes(bytes, &:sheets)
      expect(result.value.first).to have_attributes(kind: "unknown", state: "unknown")
    end
  end
end
