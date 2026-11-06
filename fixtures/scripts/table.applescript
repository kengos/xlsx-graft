-- Makes fixtures/table.xlsx in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/table.applescript <output .xlsx path>
--
-- The output path must be one Excel may write without asking, such as a
-- folder under ~/Library/Containers/com.microsoft.Excel/Data/, and no file
-- may be there yet: Excel would ask whether to replace it.
--
-- The calculated column's formula is pasted from the clipboard, which
-- Excel takes as typed, so it fills the column as a calculated column.
-- The clipboard's text is restored at the end.

on run argv
	set outPath to item 1 of argv
	set saved to the clipboard
	tell application "Microsoft Excel"
		activate
		set wb to make new workbook
		set ws to worksheet 1 of wb
		set name of ws to "Data"
		set value of range "A1:C4" of ws to {{"Item", "Qty", "Price"}, {"A", 2, 10}, {"B", 3, 20}, {"C", 5, 30}}
		set lo to make new list object at ws with properties {range object:range "A1:C4" of ws}
		set name of lo to "Items"
		set value of range "D1" of ws to "Total"
		select range "D2" of ws
	end tell
	set the clipboard to "=[@Qty]*[@Price]"
	tell application "Microsoft Excel"
		paste worksheet ws
		set total of list object "Items" of ws to true
		-- The totals row label is the UI language's word for total; a fixed one keeps the workbook the same in every language.
		set value of range "A5" of ws to "Sum"
	end tell
	set the clipboard to "=SUM(Items[Total])"
	tell application "Microsoft Excel"
		select range "F1" of ws
		paste worksheet ws
		select range "A1" of ws
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
	set the clipboard to saved
end run
