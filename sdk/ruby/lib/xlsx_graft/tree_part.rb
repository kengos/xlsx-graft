# frozen_string_literal: true

module XlsxGraft
  # One part of the package, in {Tree#parts}.
  #
  # `name` is such as `/xl/workbook.xml`. `content_type` is nil when the
  # content types stream gives the part none; when `content_type_known` is
  # false that nil does not mean the part has none. `size` and
  # `compressed_size` are the bytes the zip's central directory declares for
  # the entry, inflated and as stored; when `readable` is false (the package
  # reader did not accept the entry) they may be more than a double holds
  # exactly. `referenced` is true when a listed relationship targets the part;
  # for any other part it is false when {Tree#complete} is true, and nil when
  # it is not. `relationships_known` is false when the part's relationships
  # may include one that is not in `relationships`.
  TreePart = Data.define(:name, :content_type, :content_type_known, :size, :compressed_size,
                         :readable, :referenced, :relationships_known, :relationships)
end
