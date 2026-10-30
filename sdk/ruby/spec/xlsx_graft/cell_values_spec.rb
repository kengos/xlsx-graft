# frozen_string_literal: true

RSpec.describe XlsxGraft::CellValues do
  describe ".call" do
    let(:text) { Fiddle::Pointer["#N/A"] }

    # The values whose fields are `rows`, each a Hash of the fields that
    # differ from zero, read in `timezone`.
    def values(*rows, timezone: "UTC")
      bytes = rows.map do |row|
        fields = { kind: 0, year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0,
                   boolean: 0, millisecond: 0, reserved: 0, number: 0.0, elapsed: 0,
                   text: 0, text_len: 0 }.merge(row)
        fields.values.pack(described_class::LAYOUT)
      end.join
      described_class.call(Fiddle::Pointer[bytes].to_i, rows.size, timezone)
    end

    it "reads each kind into a Ruby value" do
      moment = { year: 2024, month: 1, day: 31, hour: 13, minute: 45, second: 30, millisecond: 250 }
      expect(values({}, { kind: 1, text: text.to_i, text_len: 4 }, { kind: 2, number: 3.0 },
                    { kind: 3, boolean: 1 }, { kind: 3 }, moment.merge(kind: 4),
                    moment.merge(kind: 5), moment.merge(kind: 6),
                    { kind: 7, elapsed: 129_600_500 }, { kind: 8, text: text.to_i, text_len: 4 }))
        .to eq([nil, "#N/A", 3.0, true, false, Date.new(2024, 1, 31),
                XlsxGraft::TimeOfDay.new(13, 45, 30, 250),
                Time.utc(2024, 1, 31, 13, 45, Rational(121, 4)), 129_600.5,
                XlsxGraft::CellError.new("#N/A")])
    end

    it "reads a date-time as the wall-clock time in the zone asked for" do
      time = values({ kind: 6, year: 2024, month: 1, day: 31, hour: 9 }, timezone: "+09:00").first
      expect(time).to have_attributes(hour: 9, utc_offset: 9 * 3600, utc?: false)
    end

    it "reads an empty text lent as NULL" do
      expect(values({ kind: 1 }, { kind: 8 })).to eq(["", XlsxGraft::CellError.new("")])
    end

    it "refuses a kind past the nine as unknown_value" do
      expect { values({ kind: 9 }) }.to raise_error(
        an_instance_of(described_class::Refused).and(having_attributes(code: "unknown_value"))
      )
    end

    it "refuses a date-time the zone's rules cannot place as datetime_unrepresentable" do
      # As TZInfo raises PeriodNotFound for a time daylight saving skips, and
      # AmbiguousTime for one it has twice.
      zone = Object.new
      def zone.local_to_utc(_time)
        raise IndexError, "skipped by daylight saving"
      end

      refused = having_attributes(code: "datetime_unrepresentable",
                                  message: start_with("2024-03-10 02:00:00.000 cannot"))
      expect { values({ kind: 6, year: 2024, month: 3, day: 10, hour: 2 }, timezone: zone) }
        .to raise_error(an_instance_of(described_class::Refused).and(refused))
    end

    it "lets a zone object's own bug through" do
      zone = Object.new
      expect { values({ kind: 6, year: 2024, month: 1, day: 1 }, timezone: zone) }
        .to raise_error(TypeError)
    end

    it "reads no values from a row the sheet writes empty" do
      expect(described_class.call(0, 0, "UTC")).to eq([])
    end
  end
end
