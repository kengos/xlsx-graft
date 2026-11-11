# frozen_string_literal: true

RSpec.describe XlsxGraft::Verifier::Policy do
  describe ".call" do
    it "writes the keys, nested ones and those inside Arrays too, in camelCase" do
      policy = { max_xml_depth: 512, forbid: { functions: [{ name: "HYPERLINK", reason: "links" }],
                                               parts: [{ kind: "external_link" }] } }
      expect(described_class.call(policy)).to eq(
        '{"maxXmlDepth":512,"forbid":{"functions":[{"name":"HYPERLINK","reason":"links"}],' \
        '"parts":[{"kind":"external_link"}]}}'
      )
    end

    it "leaves a camelCase key and the values as they are" do
      expect(described_class.call({ "failOn" => "warn", "maxEntries" => 7 }))
        .to eq('{"failOn":"warn","maxEntries":7}')
    end

    it "refuses a policy that is not a Hash and what JSON cannot write" do
      expect { described_class.call("{}") }.to raise_error(ArgumentError, /not String/)
      expect { described_class.call({ max_entries: Float::NAN }) }
        .to raise_error(ArgumentError, /cannot be written as JSON/)
    end
  end
end
