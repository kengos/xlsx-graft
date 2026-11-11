# frozen_string_literal: true

RSpec.describe XlsxGraft::Verifier do
  let(:verifier) { described_class.new }
  let(:macro) { File.join(FIXTURES, "macro.xlsm") }
  let(:table) { File.join(FIXTURES, "table.xlsx") }
  let(:dynamic) { File.join(FIXTURES, "dynamic-array.xlsx") }

  def codes(result)
    result.diagnostics.map(&:code)
  end

  describe "#verify" do
    it "fails a workbook the engine refuses, with its diagnostics and no value" do
      result = verifier.verify(macro)
      expect(result).to have_attributes(success?: false, error: nil, value: nil)
      expect(codes(result)).to eq(%w[part_denied part_denied])
      expect(result.diagnostics.first).to have_attributes(
        severity: "error", args: be_a(Hash), fix: include("in" => "template")
      )
    end

    it "lets a part through that is allowed, and takes a Pathname" do
      result = verifier.verify(Pathname(macro), allow_parts: ["macro"])
      expect(result).to have_attributes(success?: true, diagnostics: [])
      expect(verifier.verify(table)).to have_attributes(success?: true, diagnostics: [])
    end

    it "fails with an io error for a file that cannot be read" do
      result = verifier.verify("/nowhere/book.xlsx")
      expect(result).to have_attributes(success?: false, diagnostics: [])
      expect(result.error).to have_attributes(code: "io", message: include("/nowhere/book.xlsx"))
    end

    it "reports a fault of the rules as diagnostics and does not open the workbook" do
      rules = { targets: ["excel 1990"], allow_functions: ["_xlfn.SEQUENCE"] }
      result = verifier.verify("/nowhere/book.xlsx", **rules)
      expect(result).to have_attributes(success?: false, error: nil)
      expect(codes(result)).to contain_exactly("target_invalid", "rules_invalid")
      expect(result.diagnostics.find { |d| d.code == "rules_invalid" }.args)
        .to include("reason" => "prefixed")
    end

    it "fails on a warning only when fail_on says so" do
      lenient = verifier.verify(dynamic, best_effort: ["excel 2016"])
      expect(lenient).to have_attributes(success?: true)
      expect(lenient.diagnostics.map(&:severity)).to include("warn")
      [{ fail_on: "warn" }, { fail_on: :warn }].each do |rules|
        result = verifier.verify(dynamic, best_effort: ["excel 2016"], **rules)
        expect(result).to have_attributes(success?: false, error: nil)
        expect(result.diagnostics.map(&:severity)).not_to include("error")
      end
    end

    it "reads a file whose name is not UTF-8 and verifies its bytes" do
      Dir.mktmpdir do |dir|
        name = File.join(dir.b, "\xFF.xlsx".b)
        FileUtils.cp(table, name)
        expect(verifier.verify(name)).to have_attributes(success?: true)
        latin1 = File.join(dir.b, "\xE9.xlsx".b).force_encoding(Encoding::ISO_8859_1)
        FileUtils.cp(table, latin1.b)
        expect(verifier.verify(latin1)).to have_attributes(success?: true)
        result = verifier.verify(File.join(dir.b, "\xFE.xlsx".b))
        expect(result.error).to have_attributes(code: "io", message: include("could not be read"))
      end
    end

    it "raises ArgumentError for rules that are not valid UTF-8 JSON text" do
      expect { verifier.verify(table, targets: ["\xFF".b]) }.to raise_error(ArgumentError)
    end

    it "refuses options of more than 2 MiB" do
      expect { verifier.verify(table, allow_functions: ["A" * (2 << 20)]) }
        .to raise_error(ArgumentError, /the engine takes 2097152/)
    end
  end

  describe "#verify_bytes" do
    it "verifies the bytes; bytes that are not a workbook fail with a diagnostic and no error" do
      expect(verifier.verify_bytes(File.binread(table))).to have_attributes(success?: true)
      result = verifier.verify_bytes("")
      expect(result).to have_attributes(success?: false, error: nil)
      expect(result.diagnostics).not_to be_empty
    end

    it "may be shared by threads that call it at once" do
      bytes = File.binread(macro)
      results = Array.new(4) { Thread.new { codes(verifier.verify_bytes(bytes)) } }.map(&:value)
      expect(results).to all(eq(%w[part_denied part_denied]))
    end
  end

  describe ".new" do
    it "writes the policy to the engine in camelCase, so its maxEntries fails the verification" do
      result = described_class.new(policy: { max_entries: 1 }).verify(table)
      expect(result).to have_attributes(success?: false)
      expect(result.diagnostics.first.decided_by).to include("source" => "policy")
      same = described_class.new(policy: JSON.parse('{"maxEntries": 1}')).verify(table)
      expect(codes(same)).to eq(codes(result))
    end

    it "lets the policy's fail_on make a warning fail" do
      verifier = described_class.new(policy: { fail_on: "warn" })
      result = verifier.verify(dynamic, best_effort: ["excel 2016"])
      expect(result).to have_attributes(success?: false)
    end

    it "keeps the policy it checked when the caller changes theirs" do
      policy = { max_entries: 1 }
      verifier = described_class.new(policy:)
      policy[:max_entries] = 10_000
      expect(verifier.verify(table)).to have_attributes(success?: false)
    end

    it "raises PolicyInvalid with the engine's diagnostics" do
      policy = { max_entries: 0, forbid: { parts: [{ kind: "nope" }] } }
      expect { described_class.new(policy:) }.to raise_error(XlsxGraft::PolicyInvalid) do |e|
        expect(e.diagnostics.map(&:code)).to all(eq("policy_invalid"))
        expect(e.diagnostics.map { |d| d.args["key"] }).to eq(%w[maxEntries forbid.parts[0]])
        expect(e.message).to start_with("the policy is not valid: ")
      end
    end

    it "raises ArgumentError for a policy that is no Hash or cannot be written" do
      expect { described_class.new(policy: "{}") }.to raise_error(ArgumentError)
      expect { described_class.new(policy: { max_entries: Float::NAN }) }
        .to raise_error(ArgumentError)
    end
  end
end
