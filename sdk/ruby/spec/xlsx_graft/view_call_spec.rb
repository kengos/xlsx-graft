# frozen_string_literal: true

RSpec.describe XlsxGraft::ViewCall do
  let(:call) { described_class.new(XlsxGraft::Library.open) }
  let(:merged) { File.join(FIXTURES, "merged.xlsx") }

  it "returns the status and the view's JSON, for a path and bytes of each view" do
    status, json = call.call(:describe_sheet_path, merged, "")
    expect(status).to eq(0)
    expect(json).to start_with('{"ok":true')
    expect(call.call(:describe_sheet_bytes, File.binread(merged), "")).to eq([0, json])
    expect(call.call(:tree_bytes, File.binread(merged), "")).to match([0, start_with('{"ok":true')])
    expect(call.call(:tree_path, merged, "")).to match([0, start_with('{"ok":true')])
  end

  it "returns the engine's message with request_invalid and no JSON for options it refuses" do
    status, message = call.call(:describe_sheet_path, merged, '{"maxRows":0}')
    expect(status).to eq(11)
    expect(message).to include("maxRows 0")
  end

  it "returns invalid_arg and no JSON for options that are not an object" do
    expect(call.call(:describe_sheet_path, merged, "[]")).to eq([4, nil])
  end
end
