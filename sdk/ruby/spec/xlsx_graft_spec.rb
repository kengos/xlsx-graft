# frozen_string_literal: true

RSpec.describe XlsxGraft do
  describe ".version" do
    let(:engine_version) do
      cargo_toml = File.expand_path("../../../engine/Cargo.toml", __dir__)
      File.read(cargo_toml)[/^version = "([^"]+)"/, 1]
    end

    it "returns the engine's version from the library" do
      result = described_class.version
      expect(result).to have_attributes(success?: true, value: engine_version, diagnostics: [])
      expect(result.value.encoding).to eq(Encoding::UTF_8)
    end

    it "tracks the engine's version in the gem's own" do
      expect(XlsxGraft::VERSION).to eq(engine_version)
    end

    it "passes library: on to Library.open" do
      path = "/elsewhere/libxlsx_graft_capi.so"
      allow(XlsxGraft::Library).to receive(:open).and_call_original
      described_class.version(library: path)
      expect(XlsxGraft::Library).to have_received(:open).with(path: path)
    end
  end
end
