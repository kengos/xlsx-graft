package xlsxgraft

import (
	"errors"
	"fmt"
	"math"
	"regexp"
	"strconv"
	"time"
)

// Row is one row a sheet writes: its number, from 1, and the value of each
// column from A to its last cell, a column without one nil.
//
// A value is nil, a string, a float64 (3.0 included), a bool, a [Date], a
// [TimeOfDay], a [time.Time] (a date-time, in the location the reader was
// given), a [time.Duration] (an elapsed time, not wrapped at a day) or a
// [CellError].
type Row struct {
	Number int
	Values []any
}

// Date is a date with no time of day, as a workbook writes one: a
// [time.Time] would put it at a midnight in some zone.
type Date struct {
	Year  int
	Month time.Month
	Day   int
}

// String is the date in ISO 8601, as 2024-01-31.
func (d Date) String() string {
	return fmt.Sprintf("%04d-%02d-%02d", d.Year, d.Month, d.Day)
}

// TimeOfDay is a time of day with no date, as a workbook writes one: a
// [time.Time] would put it on a day the workbook never gave, and a
// [time.Duration] would be taken for an elapsed time.
type TimeOfDay struct {
	Hour, Minute, Second, Millisecond int
}

// String is the time in ISO 8601, as 18:00:00.000.
func (t TimeOfDay) String() string {
	return fmt.Sprintf("%02d:%02d:%02d.%03d", t.Hour, t.Minute, t.Second, t.Millisecond)
}

// CellError is an error value a cell holds, as written (#N/A): never a
// string, which a cell holding the text #N/A would be too.
type CellError struct {
	Text string
}

// String is the error value as written.
func (e CellError) String() string {
	return e.Text
}

var (
	datePattern     = regexp.MustCompile(`^(\d{4})-(\d{2})-(\d{2})$`)
	timePattern     = regexp.MustCompile(`^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$`)
	dateTimePattern = regexp.MustCompile(`^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d{3})$`)
)

// cellValues is the values of a `read` row line's cells; or why one of them
// cannot be given, as a diagnostic: of a type this package does not know (a
// CLI newer than the package, `unknown_value`), or a date-time loc skips or
// has twice, or an elapsed time beyond a time.Duration
// (`datetime_unrepresentable`). Neither is guessed at. A cell not of the
// contract's shape is kept in f.
func cellValues(f *fields, cells []object, loc *time.Location) ([]any, *Diagnostic) {
	values := make([]any, 0, len(cells))
	for _, cell := range cells {
		value, stop := cellValue(f, cell, loc)
		if stop != nil {
			return nil, stop
		}
		values = append(values, value)
	}
	return values, nil
}

func cellValue(f *fields, cell object, loc *time.Location) (any, *Diagnostic) {
	has(f, cell, "value")
	switch kind := get[string](f, cell, "type"); kind {
	case "empty":
		if !isNull(cell["value"]) {
			f.fail(errors.New("an empty cell's value is not null"))
		}
		return nil, nil
	case "string":
		return get[string](f, cell, "value"), nil
	case "number":
		return get[float64](f, cell, "value"), nil
	case "boolean":
		return get[bool](f, cell, "value"), nil
	case "date":
		p := parts(f, cell, kind, datePattern)
		return Date{p[0], time.Month(p[1]), p[2]}, nil
	case "time":
		p := parts(f, cell, kind, timePattern)
		return TimeOfDay{p[0], p[1], p[2], p[3]}, nil
	case "datetime":
		return moment(parts(f, cell, kind, dateTimePattern), loc)
	case "elapsed":
		return elapsed(get[int64](f, cell, "value"))
	case "error":
		return CellError{get[string](f, cell, "value")}, nil
	default:
		if f.err != nil {
			return nil, nil
		}
		d := problem("unknown_value", fmt.Sprintf("the CLI returned a value of type %s, which this package does not know", kind))
		return nil, &d
	}
}

// parts is the numbers of cell's value, a string pattern matches whole; as
// many zeros when it is not one.
func parts(f *fields, cell object, kind string, pattern *regexp.Regexp) []int {
	found := pattern.FindStringSubmatch(get[string](f, cell, "value"))
	numbers := make([]int, pattern.NumSubexp())
	if found == nil {
		f.fail(fmt.Errorf("a %s value is not of its form", kind))
		return numbers
	}
	for i := range numbers {
		// Digits alone, a few of them: they always parse.
		numbers[i], _ = strconv.Atoi(found[i+1])
	}
	return numbers
}

// moment is the instant the wall clock wall (year to millisecond) is in loc.
// time.Date places a wall clock loc's daylight saving skips or has twice
// without a word (moving it on, or taking one), so each offset loc has a day
// either side is tried, and the wall clock is placed only when exactly one
// gives it. A zone changing its offset twice within two days is not told
// apart.
func moment(wall []int, loc *time.Location) (any, *Diagnostic) {
	asUTC := time.Date(wall[0], time.Month(wall[1]), wall[2], wall[3], wall[4], wall[5], wall[6]*int(time.Millisecond), time.UTC)
	var found []time.Time
	for _, offset := range offsets(asUTC, loc) {
		placed := asUTC.Add(-time.Duration(offset) * time.Second).In(loc)
		if _, at := placed.Zone(); at == offset {
			found = append(found, placed)
		}
	}
	if len(found) == 1 {
		return found[0], nil
	}
	why := "it has it twice"
	if len(found) == 0 {
		why = "its daylight saving skips it"
	}
	shown := Date{wall[0], time.Month(wall[1]), wall[2]}.String() + " " + TimeOfDay{wall[3], wall[4], wall[5], wall[6]}.String()
	d := problem("datetime_unrepresentable", fmt.Sprintf("%s cannot be placed in %s: %s", shown, loc, why))
	return nil, &d
}

// offsets is how far ahead of UTC loc's wall clock is, in seconds, a day
// before and a day after the instant asUTC, each once.
func offsets(asUTC time.Time, loc *time.Location) []int {
	_, before := asUTC.Add(-24 * time.Hour).In(loc).Zone()
	_, after := asUTC.Add(24 * time.Hour).In(loc).Zone()
	if before == after {
		return []int{before}
	}
	return []int{before, after}
}

// elapsed is ms milliseconds as a time.Duration, which holds about 292 years
// either way: an elapsed time is never wrapped at a day, and a workbook's can
// run to 9999 years.
func elapsed(ms int64) (any, *Diagnostic) {
	if ms > math.MaxInt64/int64(time.Millisecond) || ms < math.MinInt64/int64(time.Millisecond) {
		d := problem("datetime_unrepresentable", fmt.Sprintf("an elapsed time of %d ms is beyond a time.Duration's 292 years", ms))
		return nil, &d
	}
	return time.Duration(ms) * time.Millisecond, nil
}
