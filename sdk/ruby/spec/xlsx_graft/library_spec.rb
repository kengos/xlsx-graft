# frozen_string_literal: true

RSpec.describe XlsxGraft::Library do
  describe ".open" do
    let(:dir) { Dir.mktmpdir }
    let(:packaged_dir) { File.join(dir, "native") }

    before do
      Dir.mkdir(packaged_dir)
      stub_const("#{described_class}::PACKAGED_DIR", packaged_dir)
    end

    after do
      FileUtils.remove_entry(dir)
    end

    it "takes XLSX_GRAFT_LIBRARY over the path argument" do
      env = { "XLSX_GRAFT_LIBRARY" => ENGINE_LIBRARY }
      expect(described_class.open(path: "/nowhere.so", env: env).path).to eq(ENGINE_LIBRARY)
    end

    it "takes the path argument when XLSX_GRAFT_LIBRARY is empty" do
      library = described_class.open(path: ENGINE_LIBRARY, env: { "XLSX_GRAFT_LIBRARY" => "" })
      expect(library.path).to eq(ENGINE_LIBRARY)
    end

    context "when the gem ships the library" do
      before do
        FileUtils.cp(ENGINE_LIBRARY, File.join(packaged_dir, "libxlsx_graft_capi.so"))
      end

      it "falls back to the packaged copy" do
        library = described_class.open(path: "", env: {})
        expect(library.path).to eq(File.join(packaged_dir, "libxlsx_graft_capi.so"))
      end
    end

    it "raises LibraryNotFound with the ways to install when nothing is found" do
      expect { described_class.open(env: {}) }.to raise_error(
        XlsxGraft::LibraryNotFound, /no engine library was found.*XLSX_GRAFT_LIBRARY/m
      )
    end

    context "when the file is not a library" do
      before do
        File.write(File.join(dir, "not_a_library.so"), "text")
      end

      it "raises LibraryNotFound naming the file" do
        expect { described_class.open(path: File.join(dir, "not_a_library.so"), env: {}) }
          .to raise_error(XlsxGraft::LibraryNotFound, /not_a_library\.so could not be loaded/)
      end
    end
  end

  describe "#function" do
    let(:library) { described_class.open(env: { "XLSX_GRAFT_LIBRARY" => ENGINE_LIBRARY }) }

    it "raises LibraryNotFound when the library does not export the name" do
      expect { library.function("xlsx_graft_nothing", [], Fiddle::TYPE_INT32_T) }
        .to raise_error(XlsxGraft::LibraryNotFound, /exports no `xlsx_graft_nothing`/)
    end
  end
end
