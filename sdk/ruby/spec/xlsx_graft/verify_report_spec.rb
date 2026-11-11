# frozen_string_literal: true

RSpec.describe XlsxGraft::VerifyReport do
  let(:record) do
    '{"code":"part_denied","severity":"error","message":"m","args":{},"location":{},' \
      '"decidedBy":null,"fix":null,"origin":"x"}'
  end

  def report(is_ok:, error: "null", records: "")
    %({"ok":#{is_ok},"failOn":"error","counts":{},"error":#{error},"diagnostics":[#{records}]})
  end

  it "is a success with no value when ok, whatever diagnostics it has" do
    result = described_class.call(0, report(is_ok: true))
    expect(result).to have_attributes(success?: true, value: nil, error: nil, diagnostics: [])
  end

  it "fails with the engine's diagnostics, and the result's io error as Result#error" do
    result = described_class.call(0, report(is_ok: false, records: record))
    expect(result).to have_attributes(success?: false, error: nil)
    expect(result.diagnostics.map(&:code)).to eq(["part_denied"])
    io = described_class.call(0, report(is_ok: false, error: '{"code":"io","message":"gone"}'))
    expect(io.error).to have_attributes(code: "io", message: "gone", severity: "error")
  end

  it "turns a status other than OK and invalid_arg into a failure whose error names the status" do
    expect(described_class.call(2, nil).error).to have_attributes(code: "panic")
    expect(described_class.call(99, nil).error).to have_attributes(code: "unknown_status")
  end

  it "raises for invalid_arg, a call this gem built wrongly" do
    expect { described_class.call(4, nil) }.to raise_error(XlsxGraft::Error, /a bug of the gem/)
  end

  it "fails with diagnostics_unreadable for JSON that is not a result" do
    ["not json", "{}", report(is_ok: true, records: "1"), "[]"].each do |json|
      expect(described_class.call(0, json).error).to have_attributes(code: "diagnostics_unreadable")
    end
  end
end
