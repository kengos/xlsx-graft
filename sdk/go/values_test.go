package xlsxgraft

import (
	"fmt"
	"testing"
	"time"
)

func TestADateATimeOfDayAndAnErrorValuePrintAsWritten(t *testing.T) {
	for value, want := range map[fmt.Stringer]string{
		Date{24, time.March, 1}:      "0024-03-01",
		TimeOfDay{18, 0, 5, 7}:       "18:00:05.007",
		CellError{"#DIV/0!"}:         "#DIV/0!",
		Date{2024, time.January, 31}: "2024-01-31",
	} {
		if got := fmt.Sprint(value); got != want {
			t.Errorf("%#v prints %q, want %q", value, got, want)
		}
	}
}
