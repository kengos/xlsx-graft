# frozen_string_literal: true

# One measured run of a command, as GNU time and timeout see it:
#
#   timeout -s KILL <seconds> /usr/bin/time -v -o <file> <command>
#
# The command's output goes to a file (a pipe that no process reads would stop
# the command once the pipe's buffer is full; a reading of 1,000,000 rows
# prints enough for that), of which only the last line is kept. The command's
# standard error goes to a file of its own, of which a failed run keeps the last
# STDERR_LINES lines. A run ends in one of these statuses:
#
#   ok        the command exited 0 and (when asked) its last line said the
#             book was read
#   timeout   timeout stopped it after the whole timeout. With -s KILL, timeout
#             sends SIGKILL to its process group, itself included, so
#             compare.rb sees signal 9 (never exit 124)
#   oom       signal 9 before the timeout was up: the kernel killed it, past
#             the container's memory
#   failed    it exited with another status (:exit has it), or exited 0 but
#             GNU time's report has no peak memory or no elapsed time
#             (:report says so); :stderr has the last lines of its standard
#             error
#   mismatch  it exited 0 but its last line was not what the book gives

require "json"
require "tmpdir"

module Bench
  module Measure
    TIME = "/usr/bin/time"
    TAIL_BYTES = 64 * 1024
    STDERR_LINES = 20

    module_function

    # Runs `command` for at most `timeout` seconds. `check` is a lambda over the
    # command's last output line, or nil. Returns a Hash: :status, :exit,
    # :rss_kib and :seconds (the two last nil when GNU time wrote no report: a
    # run stopped by timeout), and :report (only on a run that exited 0 without
    # a complete report), and :stderr (only on a failed run: the last lines of
    # the command's standard error, omitted when it wrote none).
    def run(command, timeout:, check: nil)
      Dir.mktmpdir("bench") do |dir|
        out = File.join(dir, "out")
        err = File.join(dir, "err")
        report = File.join(dir, "time")
        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        pid = Process.spawn("timeout", "-s", "KILL", timeout.to_s, TIME, "-v", "-o", report,
                            *command, in: File::NULL, out: out, err: err)
        _, status = Process.wait2(pid)
        wall = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
        parsed = parse_report(report)
        finish(status, parsed, wall, timeout, check && last_line(out), check, err)
      end
    end

    def finish(status, parsed, wall, timeout, line, check, err)
      code = status.exitstatus
      killed = status.termsig == 9 || code == 137 || parsed[:signal] == 9
      outcome =
        if (killed && wall >= timeout) then :timeout
        elsif killed then :oom # wall < timeout here
        elsif code != 0 then :failed
        elsif parsed[:rss_kib].nil? || parsed[:seconds].nil? then :failed
        elsif check && !check.call(line) then :mismatch
        else :ok
        end
      run = { status: outcome, exit: code, rss_kib: parsed[:rss_kib], seconds: parsed[:seconds] }
      run[:report] = "GNU time's report has no peak memory or no elapsed time" if outcome == :failed && code == 0
      stderr = outcome == :failed ? tail_lines(err) : ""
      run[:stderr] = stderr unless stderr.empty?
      run
    end

    # The last STDERR_LINES lines of the file (from its last TAIL_BYTES bytes),
    # or "" when it is missing.
    def tail_lines(path)
      return "" unless File.exist?(path)

      text = File.open(path, "rb") do |file|
        file.seek([file.size - TAIL_BYTES, 0].max)
        file.read.to_s
      end
      text.force_encoding(Encoding::UTF_8).scrub.lines(chomp: true).last(STDERR_LINES).join("\n")
    end

    # The fields of `time -v` this needs, from its report file (empty Hash when
    # the file is missing).
    def parse_report(path)
      return {} unless File.exist?(path)

      text = File.read(path)
      {
        rss_kib: text[/Maximum resident set size \(kbytes\): (\d+)/, 1]&.to_i,
        seconds: seconds(text[/Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): ([\d:.]+)/, 1]),
        signal: text[/Command terminated by signal (\d+)/, 1]&.to_i
      }
    end

    # "1:02:03.45", "2:03.45" or "3.45" as seconds.
    def seconds(text)
      return nil if text.nil?

      text.split(":").map(&:to_f).inject(0.0) { |total, part| (total * 60) + part }
    end

    # The last non-empty line of the file, read backwards from the end in
    # growing chunks until a newline precedes it or the file starts (a line
    # may be larger than any fixed tail).
    def last_line(path)
      File.open(path, "rb") do |file|
        size = file.size
        chunk = TAIL_BYTES
        loop do
          start = [size - chunk, 0].max
          file.seek(start)
          text = file.read(size - start).to_s.rstrip
          return text.split("\n").last.to_s.strip if start.zero? || text.include?("\n")

          chunk *= 2
        end
      end
    end
  end
end
