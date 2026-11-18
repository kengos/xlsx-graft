# frozen_string_literal: true

module XlsxGraft
  # Reads the keys of the `tree` JSON after `ok`, `error` and `diagnostics`
  # into a {Tree}. A key it does not know is ignored.
  class TreeContent
    class << self
      # @param view [Hash] the parsed JSON
      # @return [Tree]
      # @raise [KeyError, TypeError, NoMethodError] for JSON that lacks a key it reads
      def call(view)
        Tree.new(
          complete: view.fetch("complete"), relationships_known: view.fetch("relationshipsKnown"),
          relationships: relationships(view.fetch("relationships")),
          parts: view.fetch("parts").map { |part| part(part) }.freeze
        )
      end

      private

      def part(part)
        TreePart.new(
          name: part.fetch("name"), content_type: part.fetch("contentType"),
          content_type_known: part.fetch("contentTypeKnown"), size: part.fetch("size"),
          compressed_size: part.fetch("compressedSize"), readable: part.fetch("readable"),
          referenced: part.fetch("referenced"),
          relationships_known: part.fetch("relationshipsKnown"),
          relationships: relationships(part.fetch("relationships"))
        )
      end

      def relationships(records)
        records.map do |record|
          target = record.fetch("target")
          TreeRelationship.new(
            id: record.fetch("id"), type: record.fetch("type"),
            target: TreeTarget.new(part: target.fetch("part", nil),
                                   external: target.fetch("external", nil))
          )
        end.freeze
      end
    end
  end
end
