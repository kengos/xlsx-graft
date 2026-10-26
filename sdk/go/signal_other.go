//go:build !unix

package xlsxgraft

import "os"

// signalOf is 0: outside unix a process is not killed by a signal.
func signalOf(*os.ProcessState) int {
	return 0
}
