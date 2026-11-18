# frozen_string_literal: true

RSpec.describe XlsxGraft::PathInput do
  let(:table) { File.join(FIXTURES, "table.xlsx") }

  def yielded(path)
    described_class.call(path) { |how, input| [how, input] }
  end

  it "yields a path of UTF-8 text as the path, and accepts a Pathname" do
    expect(yielded(table)).to eq([:path, table])
    expect(yielded(Pathname(table))).to eq([:path, table])
  end

  it "yields the bytes of the file for a path whose bytes are not UTF-8" do
    Dir.mktmpdir do |dir|
      binary = File.join(dir.b, "\xFF.xlsx".b)
      FileUtils.cp(table, binary)
      latin1 = File.join(dir.b, "\xE9.xlsx".b).force_encoding(Encoding::ISO_8859_1)
      FileUtils.cp(table, latin1.b)
      expect(yielded(binary)).to eq([:bytes, File.binread(table)])
      expect(yielded(latin1)).to eq([:bytes, File.binread(table)])
    end
  end

  it "returns an io failure and does not call the block for a file that cannot be read" do
    called = false
    result = described_class.call(File.join("/nowhere".b, "\xFE.xlsx".b)) { called = true }
    expect(called).to be(false)
    expect(result).to have_attributes(success?: false, diagnostics: [])
    expect(result.error).to have_attributes(code: "io", message: include("could not be read"))
  end
end
