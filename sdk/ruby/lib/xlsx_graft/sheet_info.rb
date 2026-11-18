# frozen_string_literal: true

module XlsxGraft
  # A sheet as the workbook lists it, from {Book#sheets} or {Grid#sheet}.
  # From a Grid the name is tidied for display (it may be cut at 200
  # characters), and a kind or state newer than the gem is the engine's
  # string as written.
  #
  # `index` counts from 0 and takes in hidden sheets and chartsheets, as
  # {Book#sheet} does. `kind` is "worksheet" (the one kind with rows to read),
  # "chartsheet", "dialogsheet", "macrosheet" or "unknown" (its part is not
  # known; the diagnostics say why). `state` is "visible", "hidden" or
  # "veryHidden" (only a program can show it again). From {Book#sheets}, a
  # kind or state newer than the gem is "unknown".
  SheetInfo = Data.define(:index, :name, :kind, :state)
end
