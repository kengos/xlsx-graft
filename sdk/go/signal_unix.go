//go:build unix

package xlsxgraft

import (
	"os"
	"syscall"
)

// signalOf is the signal that killed the process, 0 when it exited. The wait
// status is the one place that tells the two apart: ExitCode is -1 for both a
// signal and a process still running.
func signalOf(state *os.ProcessState) int {
	// A unix ProcessState always carries a syscall.WaitStatus.
	status, _ := state.Sys().(syscall.WaitStatus)
	if !status.Signaled() {
		return 0
	}
	return int(status.Signal())
}
