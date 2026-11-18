# frozen_string_literal: true

RSpec.describe XlsxGraft::ViewReport do
  let(:tree) do
    '{"ok":true,"error":null,"diagnostics":[],"complete":false,"relationshipsKnown":true,' \
      '"relationships":[{"id":"rId1","type":"t","target":{"part":"/a.xml"}}],' \
      '"parts":[{"name":"/a.xml","contentType":null,"contentTypeKnown":false,"size":3,' \
      '"compressedSize":2,"readable":true,"referenced":null,"relationshipsKnown":false,' \
      '"relationships":[{"id":"rId2","type":"x","target":{"external":"http://e/"}}],' \
      '"later":1}],"laterKey":[]}'
  end

  def grid(cells, sheet: nil, **failure)
    sheet ||= '{"index":1,"name":"S","kind":"worksheet","state":"hidden","later":2}'
    [%({"ok":#{failure.fetch(:ok, true)},"error":#{failure.fetch(:error, "null")},),
     %("diagnostics":[],"sheet":#{sheet},"dimension":null,"used":"A1:A1","shown":"A1:A1",),
     %("truncated":{"rows":false,"columns":true,"merges":false},"complete":true,),
     %("rows":[{"row":1,"cells":[#{cells}]}],"merges":[]})].join
  end

  def read(json, content = XlsxGraft::GridContent)
    described_class.call(0, json, content)
  end

  it "reads a tree, ignoring keys it does not know" do
    result = read(tree, XlsxGraft::TreeContent)
    part = result.value.parts.first
    expect(result).to have_attributes(success?: true, error: nil, diagnostics: [])
    expect(result.value).to have_attributes(complete: false, relationships_known: true)
    expect(result.value.relationships).to eq(
      [XlsxGraft::TreeRelationship.new(id: "rId1", type: "t",
                                       target: XlsxGraft::TreeTarget.new(part: "/a.xml",
                                                                         external: nil))]
    )
    expect(part).to have_attributes(content_type: nil, content_type_known: false, size: 3,
                                    compressed_size: 2, readable: true, referenced: nil)
    expect(part.relationships.first.target).to have_attributes(part: nil, external: "http://e/")
  end

  it "reads each type of cell as the JSON gives it, and a cell without truncated is whole" do
    cells = [
      '{"ref":"A1","type":"number","value":2.0}', '{"ref":"B1","type":"elapsed","value":90000}',
      '{"ref":"C1","type":"boolean","value":true}', '{"ref":"D1","type":"error","value":"#N/A"}',
      '{"ref":"E1","type":"datetime","value":"2024-01-15T08:30:00.000"}',
      '{"ref":"F1","type":"string","value":"cut…","truncated":true,"later":1}'
    ].join(",")
    result = read(grid(cells))
    values = result.value.rows.first.cells.map { |c| [c.value, c.truncated] }
    expect(values).to eq([[2.0, false], [90_000, false], [true, false], ["#N/A", false],
                          ["2024-01-15T08:30:00.000", false], ["cut…", true]])
    expect(result.value.rows.first.cells.first.value).to be_a(Float)
  end

  it "reads the sheet and the truncated flags of a grid" do
    expect(read(grid("")).value).to have_attributes(
      sheet: XlsxGraft::SheetInfo.new(index: 1, name: "S", kind: "worksheet", state: "hidden"),
      dimension: nil, truncated: XlsxGraft::GridTruncated.new(rows: false, columns: true,
                                                              merges: false)
    )
  end

  it "has a nil sheet for a null sheet, and keeps the value of a failure" do
    result = read(grid("", ok: false, error: '{"code":"io","message":"gone"}', sheet: "null"))
    expect(result).to have_attributes(success?: false, value: be_a(XlsxGraft::Grid))
    expect(result.error).to have_attributes(code: "io", message: "gone")
    expect(result.value.sheet).to be_nil
  end

  it "raises ArgumentError for request_invalid, with the message, and Error for invalid_arg" do
    expect { described_class.call(11, "maxRows 0 is not 1 to 10000", XlsxGraft::GridContent) }
      .to raise_error(ArgumentError, "maxRows 0 is not 1 to 10000")
    expect { described_class.call(4, nil, XlsxGraft::GridContent) }
      .to raise_error(XlsxGraft::Error, /a bug of the gem/)
  end

  it "raises ArgumentError with a fixed message for request_invalid with no message" do
    [nil, ""].each do |text|
      expect { described_class.call(11, text, XlsxGraft::GridContent) }
        .to raise_error(ArgumentError, /gave no reason/)
    end
  end

  it "fails with diagnostics_unreadable for a relationship target that is not an object" do
    json = tree.sub('"target":{"part":"/a.xml"}', '"target":"/a.xml"')
    result = read(json, XlsxGraft::TreeContent)
    expect(result.error).to have_attributes(code: "diagnostics_unreadable")
    expect(result.value).to be_nil
  end

  it "turns a status other than OK, request_invalid and invalid_arg into a failure with no value" do
    panic = described_class.call(2, nil, XlsxGraft::GridContent)
    unknown = described_class.call(99, nil, XlsxGraft::GridContent)
    expect(panic).to have_attributes(success?: false, value: nil)
    expect(panic.error).to have_attributes(code: "panic")
    expect(unknown.error).to have_attributes(code: "unknown_status")
  end

  it "fails with diagnostics_unreadable and no value for JSON that is not a view" do
    shapes = [grid("").sub('"merges":[]', '"merges":5'), grid("", sheet: "false")]
    ["not json", "{}", '{"ok":true}', "[]", *shapes].each do |json|
      result = read(json)
      expect(result.error).to have_attributes(code: "diagnostics_unreadable")
      expect(result.value).to be_nil
    end
  end
end
