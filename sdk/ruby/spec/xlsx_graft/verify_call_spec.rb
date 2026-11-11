# frozen_string_literal: true

RSpec.describe XlsxGraft::VerifyCall do
  let(:call) { described_class.new(XlsxGraft::Library.open) }
  let(:options) { '{"policy":null,"rules":{}}' }

  it "returns the status and the result's JSON, for a path, bytes and a policy" do
    table = File.join(FIXTURES, "table.xlsx")
    clean = '{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},' \
            '"error":null,"diagnostics":[]}'
    expect(call.path(table, options)).to eq([0, clean])
    expect(call.bytes(File.binread(table), options)).to eq([0, clean])
    expect(call.policy("{}")).to eq([0, clean])
  end

  it "returns invalid_arg and no JSON for options that are not an object of policy and rules" do
    expect(call.bytes("", "[]")).to eq([4, nil])
  end
end
