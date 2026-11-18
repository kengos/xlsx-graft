# frozen_string_literal: true

module XlsxGraft
  # Reads the keys of the `describe_sheet` JSON after `ok`, `error` and
  # `diagnostics` into a {Grid}. A key it does not know is ignored.
  class GridContent
    class << self
      # @param view [Hash] the parsed JSON
      # @return [Grid]
      # @raise [KeyError, TypeError, NoMethodError] for JSON that lacks a key it reads
      def call(view)
        Grid.new(
          sheet: sheet(view.fetch("sheet")), dimension: view.fetch("dimension"),
          used: view.fetch("used"), shown: view.fetch("shown"),
          truncated: truncated(view.fetch("truncated")), complete: view.fetch("complete"),
          rows: view.fetch("rows").map { |row| row(row) }.freeze,
          merges: view.fetch("merges").to_ary.freeze
        )
      end

      private

      def sheet(sheet)
        return if sheet.nil?

        SheetInfo.new(index: sheet.fetch("index"), name: sheet.fetch("name"),
                      kind: sheet.fetch("kind"), state: sheet.fetch("state"))
      end

      def truncated(truncated)
        GridTruncated.new(rows: truncated.fetch("rows"), columns: truncated.fetch("columns"),
                          merges: truncated.fetch("merges"))
      end

      def row(row)
        GridRow.new(row: row.fetch("row"), cells: row.fetch("cells").map { |c| cell(c) }.freeze)
      end

      def cell(cell)
        GridCell.new(ref: cell.fetch("ref"), type: cell.fetch("type"), value: cell.fetch("value"),
                     truncated: cell.fetch("truncated", false))
      end
    end
  end
end
