# frozen_string_literal: true

module XlsxGraft
  # A sheet as the workbook lists it, from {Book#sheets}.
  #
  # `index` counts from 0 and takes in hidden sheets and chartsheets, as
  # {Book#sheet} does. `kind` is "worksheet" (the one kind with rows to read),
  # "chartsheet", "dialogsheet", "macrosheet" or "unknown" (its part is not
  # known; the diagnostics say why). `state` is "visible", "hidden" or
  # "veryHidden" (only a program can show it again). A kind or state newer
  # than the gem is "unknown".
  SheetInfo = Data.define(:index, :name, :kind, :state)
end
