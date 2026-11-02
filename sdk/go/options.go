package xlsxgraft

import "time"

// Option configures a call.
type Option func(*options)

type options struct {
	cli      string
	location *time.Location
}

// WithCLI runs the CLI at path when XLSX_GRAFT_CLI is not set. A name with
// no directory in it is looked up on PATH.
func WithCLI(path string) Option {
	return func(o *options) {
		o.cli = path
	}
}

// WithLocation reads a date-time cell, the workbook's wall-clock time with no
// zone, in loc rather than in UTC. A nil loc is UTC.
func WithLocation(loc *time.Location) Option {
	return func(o *options) {
		o.location = loc
	}
}

func apply(opts []Option) options {
	o := options{location: time.UTC}
	for _, opt := range opts {
		opt(&o)
	}
	if o.location == nil {
		o.location = time.UTC
	}
	return o
}
