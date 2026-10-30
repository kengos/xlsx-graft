# frozen_string_literal: true

require "zlib"

# Workbooks for the specs to read, built as a spec asks: a zip of stored
# entries holding a package with a shared strings table of "a" and "b",
# styles whose cell formats 1 to 4 are a date, a time, a date-time and an
# elapsed time, and the sheets the spec gives. The same workbook as
# engine/capi/src/tests/workbook.rs builds, written again in Ruby.
module Workbook
  NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
  TYPES = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
  RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
  CHARTSHEET = "#{TYPES}/chartsheet".freeze
  WORKSHEET = "#{TYPES}/worksheet".freeze
  CONTENT_TYPES =
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">' \
    '<Default Extension="rels" ' \
    'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>' \
    '<Default Extension="xml" ContentType="application/xml"/></Types>'

  # A sheet: its name, its `state` (nil: none written), the type of its
  # relationship (nil: its r:id names none) and its part's XML.
  Sheet = Data.define(:name, :state, :rel_type, :xml)

  module_function

  # A worksheet named `name` whose sheetData holds `rows`.
  def worksheet(name, rows = "", state: nil)
    Sheet.new(name:, state:, rel_type: WORKSHEET,
              xml: %(<worksheet xmlns="#{NS}"><sheetData>#{rows}</sheetData></worksheet>))
  end

  # A chartsheet named `name`.
  def chartsheet(name)
    Sheet.new(name:, state: nil, rel_type: CHARTSHEET, xml: %(<chartsheet xmlns="#{NS}"/>))
  end

  # A sheet named `name` whose r:id names no relationship.
  def unrelated(name)
    Sheet.new(name:, state: nil, rel_type: nil, xml: "")
  end

  # The bytes of a workbook of `sheets`, in order, each at
  # xl/sheets/sheet<n>.xml.
  def xlsx(*sheets)
    numbered = sheets.each.with_index(1)
    entries = {
      "[Content_Types].xml" => CONTENT_TYPES,
      "_rels/.rels" => rels(rel("rId1", "#{TYPES}/officeDocument", "xl/workbook.xml")),
      "xl/workbook.xml" => workbook(numbered),
      "xl/_rels/workbook.xml.rels" => rels(related(numbered)),
      "xl/sst.xml" => %(<sst xmlns="#{NS}"><si><t>a</t></si><si><t>b</t></si></sst>),
      "xl/styles.xml" => styles
    }
    numbered.each { |sheet, n| entries["xl/sheets/sheet#{n}.xml"] = sheet.xml if sheet.rel_type }
    zip(entries)
  end

  # `bytes` written to book.xlsx in a new directory, which the block gets
  # the path of and which is removed when it ends.
  def write(bytes)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "book.xlsx")
      File.binwrite(path, bytes)
      yield path
    end
  end

  def workbook(numbered)
    listed = numbered.map do |sheet, n|
      state = sheet.state ? %( state="#{sheet.state}") : ""
      %(<sheet name="#{sheet.name}" sheetId="#{n}"#{state} r:id="rId#{n}"/>)
    end
    %(<workbook xmlns="#{NS}" xmlns:r="#{TYPES}"><sheets>#{listed.join}</sheets></workbook>)
  end

  def related(numbered)
    sheets = numbered.select { |sheet, _| sheet.rel_type }
                     .map { |sheet, n| rel("rId#{n}", sheet.rel_type, "sheets/sheet#{n}.xml") }
    rel("rIdS", "#{TYPES}/sharedStrings", "sst.xml") +
      rel("rIdT", "#{TYPES}/styles", "styles.xml") + sheets.join
  end

  def styles
    formats = [0, 14, 21, 22, 46].map { |id| %(<xf numFmtId="#{id}"/>) }.join
    %(<styleSheet xmlns="#{NS}"><cellXfs>#{formats}</cellXfs></styleSheet>)
  end

  def rel(id, type, target)
    %(<Relationship Id="#{id}" Type="#{type}" Target="#{target}"/>)
  end

  def rels(body)
    %(<Relationships xmlns="#{RELS}">#{body}</Relationships>)
  end

  # The zip of `entries`, each stored, in order.
  def zip(entries)
    zip = +"".b
    directory = +"".b
    entries.each do |name, data|
      offset = zip.bytesize
      header = [20, 0, 0, 0, 0, Zlib.crc32(data), data.bytesize, data.bytesize, name.bytesize]
      zip << [0x04034b50, *header, 0].pack("VvvvvvVVVvv") << name.b << data.b
      directory << [0x02014b50, 20, *header, 0, 0, 0, 0, 0, offset].pack("VvvvvvvVVVvvvvvVV")
      directory << name.b
    end
    count = entries.size
    trailer = [0x06054b50, 0, 0, count, count, directory.bytesize, zip.bytesize, 0]
    zip << directory << trailer.pack("VvvvvVVv")
  end
end
