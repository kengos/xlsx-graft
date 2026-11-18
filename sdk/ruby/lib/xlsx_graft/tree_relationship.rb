# frozen_string_literal: true

module XlsxGraft
  # One relationship of a Relationships part, in {Tree#relationships} or
  # {TreePart#relationships}: its `Id` as `id`, its `Type` (a URI, as written)
  # as `type`, and what it points to as `target` (a {TreeTarget}).
  TreeRelationship = Data.define(:id, :type, :target)
end
