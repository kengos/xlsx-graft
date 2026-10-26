package xlsxgraft

// Option configures a call.
type Option func(*options)

type options struct {
	cli string
}

// WithCLI runs the CLI at path when XLSX_GRAFT_CLI is not set. A name with
// no directory in it is looked up on PATH.
func WithCLI(path string) Option {
	return func(o *options) {
		o.cli = path
	}
}

func apply(opts []Option) options {
	var o options
	for _, opt := range opts {
		opt(&o)
	}
	return o
}
