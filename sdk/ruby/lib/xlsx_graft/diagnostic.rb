# frozen_string_literal: true

module XlsxGraft
  # One thing the engine reported. `code` is the stable, machine-readable
  # part; `message` is for people.
  #
  # The engine's own diagnostics ({Result#diagnostics} of a read) carry every
  # field of the JSON every host prints, keys as written there: `severity`
  # ("error", "warning" or "notice"), `args`, `location`, `decided_by` (its
  # `decidedBy`) and `fix`. One the gem makes from a C ABI status
  # ({StatusDiagnostic}) is an "error" with `args` and `location` empty and
  # no `decided_by` or `fix`.
  Diagnostic = Data.define(:code, :severity, :message, :args, :location, :decided_by, :fix)
end
