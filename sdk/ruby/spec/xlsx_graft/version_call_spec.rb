# frozen_string_literal: true

RSpec.describe XlsxGraft::VersionCall do
  describe ".call" do
    let(:library) do
      instance_double(XlsxGraft::Library, function: instance_double(Fiddle::Function, call: status))
    end

    context "when the engine returns XLSX_GRAFT_ERR_PANIC" do
      let(:status) { 2 }

      it "returns a failure carrying the panic diagnostic" do
        expect(described_class.call(library)).to have_attributes(
          success?: false, failure?: true, value: nil,
          diagnostics: [have_attributes(code: "panic")]
        )
      end
    end

    context "when the engine returns XLSX_GRAFT_ERR_NULL_ARG" do
      let(:status) { 1 }

      it "returns a failure carrying the null_arg diagnostic" do
        expect(described_class.call(library).diagnostics).to contain_exactly(
          have_attributes(code: "null_arg")
        )
      end
    end

    context "when the engine returns a status this gem does not know" do
      let(:status) { 42 }

      it "returns a failure naming the status" do
        expect(described_class.call(library).diagnostics).to contain_exactly(
          have_attributes(code: "unknown_status", message: include("status 42"))
        )
      end
    end
  end
end
