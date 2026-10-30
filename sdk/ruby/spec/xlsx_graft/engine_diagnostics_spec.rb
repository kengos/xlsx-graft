# frozen_string_literal: true

RSpec.describe XlsxGraft::EngineDiagnostics do
  describe ".call" do
    it "reads the first failure and every diagnostic" do
      record = { code: "cell_order_invalid", severity: "error", message: "m", args: { "row" => 1 },
                 location: { "part" => "/xl/sheets/sheet1.xml" }, decidedBy: nil, fix: nil,
                 origin: "x.rs:1" }
      json = JSON.generate(has_errors: true, error: { code: "io", message: "gone" },
                           diagnostics: [record])
      expect(described_class.call(json)).to have_attributes(
        has_errors: true,
        error: have_attributes(code: "io", severity: "error", message: "gone"),
        diagnostics: [have_attributes(code: "cell_order_invalid", args: { "row" => 1 },
                                      location: { "part" => "/xl/sheets/sheet1.xml" },
                                      decided_by: nil)]
      )
    end

    it "reads no failure as nil" do
      json = '{"has_errors":false,"error":null,"diagnostics":[]}'
      expect(described_class.call(json)).to have_attributes(has_errors: false, error: nil,
                                                            diagnostics: [])
    end
  end
end
