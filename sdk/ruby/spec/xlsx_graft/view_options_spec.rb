# frozen_string_literal: true

RSpec.describe XlsxGraft::ViewOptions do
  def options(sheet: nil, range: nil, max_rows: nil, max_columns: nil)
    described_class.call(sheet:, range:, max_rows:, max_columns:)
  end

  it "writes no key for a value that is nil" do
    expect(options).to eq("{}")
    expect(described_class.call(sheet: nil, range: nil, max_rows: nil, max_columns: nil))
      .to eq("{}")
    expect(options(sheet: "S")).to eq('{"sheet":"S"}')
    expect(options(max_columns: 3)).to eq('{"maxColumns":3}')
  end

  it "writes an Integer sheet as sheetIndex and the other sheets as sheet" do
    expect(options(sheet: 2)).to eq('{"sheetIndex":2}')
    expect(options(sheet: :Data)).to eq('{"sheet":"Data"}')
  end

  it "writes every key in the order sheet, range, maxRows, maxColumns" do
    json = options(sheet: 1, range: "A1:B2", max_rows: 5, max_columns: 6)
    expect(json).to eq('{"sheetIndex":1,"range":"A1:B2","maxRows":5,"maxColumns":6}')
  end

  it "raises ArgumentError for an Integer beyond a double, also inside an Array or a Hash" do
    expect { options(max_rows: 10**400) }.to raise_error(ArgumentError, /max_rows/)
    expect { options(sheet: -(10**400)) }.to raise_error(ArgumentError, /sheet/)
    expect { options(range: [10**400]) }.to raise_error(ArgumentError, /range/)
    expect { options(sheet: { a: 10**400 }) }.to raise_error(ArgumentError, /sheet/)
  end

  it "raises ArgumentError for a value JSON cannot write, or options of more than 2 MiB" do
    expect { options(range: "\xFF".b) }.to raise_error(ArgumentError, /JSON/)
    expect { options(max_rows: Float::INFINITY) }.to raise_error(ArgumentError, /JSON/)
    expect { options(range: "\x82".dup.force_encoding("Shift_JIS")) }.to raise_error(ArgumentError)
    expect { options(range: "A" * (2 << 20)) }.to raise_error(ArgumentError, /2097152/)
  end

  it "raises ArgumentError for a cyclic value and for one nested deeper than JSON writes" do
    cycle = []
    cycle << cycle
    deep = (1..101).reduce([]) { |inner, _| [inner] }
    expect { options(range: cycle) }.to raise_error(ArgumentError, /JSON/)
    expect { options(range: deep) }.to raise_error(ArgumentError, /JSON/)
  end
end
