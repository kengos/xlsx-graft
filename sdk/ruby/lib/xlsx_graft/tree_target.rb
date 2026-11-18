# frozen_string_literal: true

module XlsxGraft
  # What a {TreeRelationship} points to: `part` is the name of a part of the
  # package, `external` is the target of a link outside it. For the kinds of
  # target the engine writes, exactly one of `part` and `external` is set; for
  # a kind of target newer than the gem, both are nil.
  TreeTarget = Data.define(:part, :external)
end
