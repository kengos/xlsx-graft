# frozen_string_literal: true

RSpec.describe XlsxGraft::Capi do
  let(:capi) { described_class.new(XlsxGraft::Library.open) }
  let(:bytes) do
    Workbook.xlsx(
      Workbook.worksheet("Data", '<row r="2"><c r="B2" t="s"><v>1</v></c></row><row r="4"/>'),
      Workbook.chartsheet("Chart")
    )
  end

  # The next row of `rows`: its number and its values.
  def next_row(rows)
    status, number, address, count = capi.rows_next(rows)
    [status, number, XlsxGraft::CellValues.call(address, count, "UTC")]
  end

  it "opens a book from bytes, lists its sheets, reads a sheet's rows and closes" do
    status, book = capi.book_open_bytes(bytes)
    expect([status, capi.book_sheets(book)]).to eq([0, [0, [["Data", 0, 0], ["Chart", 1, 0]]]])
    status, rows = capi.rows_open_name(book, "DATA")
    expect(status).to eq(0)
    expect([next_row(rows), next_row(rows), next_row(rows)])
      .to eq([[0, 2, [nil, "b"]], [0, 4, []], [0, 0, []]])
    clean = '{"has_errors":false,"error":null,"diagnostics":[]}'
    expect(capi.book_diagnostics(book)).to eq([0, clean])
    expect([capi.rows_close(rows), capi.book_close(book)]).to eq([0, 0])
  end

  it "keeps a refused sheet as the book's error, in read's words" do
    _, book = capi.book_open_bytes(bytes)
    expect(capi.rows_open(book, 1)).to eq([8, 0])
    report = XlsxGraft::EngineDiagnostics.call(capi.book_diagnostics(book).last)
    expect(report.error).to have_attributes(
      code: "sheet_not_worksheet", message: 'sheet "Chart" is a chartsheet, which holds no cells'
    )
    capi.book_close(book)
  end

  it "closes a book's rows with it" do
    _, book = capi.book_open_bytes(bytes)
    _, rows = capi.rows_open(book, 0)
    capi.book_close(book)
    expect(capi.rows_next(rows)).to eq([10, 0, 0, 0])
    expect(capi.rows_close(rows)).to eq(0)
  end

  it "opens a book from a path" do
    Workbook.write(bytes) do |path|
      status, book = capi.book_open_path(path)
      expect([status, capi.book_sheets(book).last.size]).to eq([0, 2])
      capi.book_close(book)
    end
  end

  it "fails a path that will not open with io, and says why on the thread" do
    expect(capi.book_open_path("/nowhere/book.xlsx")).to eq([3, 0])
    expect(capi.error_message).to include("/nowhere/book.xlsx")
  end

  it "refuses text that is not UTF-8" do
    expect(capi.book_open_path("/tmp/\xFF.xlsx".b)).to eq([4, 0])
    _, book = capi.book_open_bytes(bytes)
    expect(capi.rows_open_name(book, "\xFF".b)).to eq([4, 0])
    capi.book_close(book)
  end

  it "passes no bytes as NULL, a book that cannot be read, whose diagnostics say why" do
    status, book = capi.book_open_bytes("")
    expect([status, capi.book_sheets(book)]).to eq([0, [5, []]])
    report = XlsxGraft::EngineDiagnostics.call(capi.book_diagnostics(book).last)
    expect(report).to have_attributes(
      has_errors: true, error: have_attributes(code: "workbook_unreadable"),
      diagnostics: include(have_attributes(severity: "error", args: be_a(Hash),
                                           location: be_a(Hash)))
    )
    capi.book_close(book)
  end
end
