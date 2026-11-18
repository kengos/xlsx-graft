# frozen_string_literal: true

module XlsxGraft
  # The parts of a workbook and the relationships between them, as
  # {Viewer#tree} returns them. Each attribute is the key of the same name
  # (snake_case) in the engine's `tree` JSON (docs/sdk.md § Listing parts).
  #
  # Names, ids, types, content types and external targets are values for
  # display: control characters and characters that reorder a line are
  # removed, and a value longer than 200 characters is cut and ends in "…",
  # so two different names may be the same text.
  #
  # `complete` is false when a Relationships part was not read, or a
  # relationship was not kept, that may have targeted a part.
  # `relationships_known` is false when the package's own relationships may
  # include one that is not in `relationships`. `relationships` are the
  # package's own, from `/_rels/.rels`. `parts` are in the order of their zip
  # entries.
  Tree = Data.define(:complete, :relationships_known, :relationships, :parts)
end
