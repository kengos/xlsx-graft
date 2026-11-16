-- Makes fixtures/merged.xlsx in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/merged.applescript <output .xlsx path>
--
-- The output path must be one Excel may write without asking, such as a
-- folder under ~/Library/Containers/com.microsoft.Excel/Data/, and no file
-- may be there yet: Excel would ask whether to replace it.
--
-- Sheet1 holds, to read as a grid with merged cells:
--   A1:C1  a merged heading, and A3:A5 a merged label down three rows
--   row 7  no cell, and column D no cell: a gap in the rows and in the columns
--   B4     a string with a line feed, B5 a string with a vertical bar
--   C6     a date, 2024-01-15, made by a formula
--   E2     a value after the empty column D
--   A8:C8  a total row after the empty row 7

on run argv
	set outPath to item 1 of argv
	tell application "Microsoft Excel"
		set wb to make new workbook
		set ws to worksheet 1 of wb
		set name of ws to "Sheet1"
		set value of range "A1" of ws to "Quarterly report"
		set value of range "A2:C2" of ws to {{"Group", "Item", "Amount"}}
		set value of range "A3" of ws to "Fruit"
		set value of range "B3:C3" of ws to {{"apple", 1.5}}
		set value of range "B4" of ws to "line one" & linefeed & "line two"
		set value of range "C4" of ws to 2
		set value of range "B5" of ws to "a|b"
		set value of range "C5" of ws to 3
		set value of range "B6" of ws to "date"
		set formula of range "C6" of ws to "=DATE(2024,1,15)"
		set number format of range "C6" of ws to "yyyy-mm-dd"
		set value of range "E2" of ws to "after the gap"
		set value of range "A8" of ws to "Total"
		set value of range "C8" of ws to 6.5
		merge range "A1:C1" of ws
		merge range "A3:A5" of ws
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
end run
