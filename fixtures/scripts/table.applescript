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
-- The clipboard's text is restored after the last paste, also when a
-- paste fails; what else the clipboard held (an image) is not.

on pasteFormula(ws, cellName, formulaText)
	tell application "Microsoft Excel" to select range cellName of ws
	set the clipboard to formulaText
	tell application "Microsoft Excel" to paste worksheet ws
end pasteFormula

on run argv
	set outPath to item 1 of argv
	set saved to ""
	try
		set saved to the clipboard as text
	end try
	tell application "Microsoft Excel"
		activate
		set wb to make new workbook
		set ws to worksheet 1 of wb
		set name of ws to "Data"
		set value of range "A1:C4" of ws to {{"Item", "Qty", "Price"}, ¬
			{"A", 2, 10}, {"B", 3, 20}, {"C", 5, 30}}
		set lo to make new list object at ws with properties ¬
			{range object:range "A1:C4" of ws}
		set name of lo to "Items"
		set value of range "D1" of ws to "Total"
	end tell
	try
		pasteFormula(ws, "D2", "=[@Qty]*[@Price]")
		pasteFormula(ws, "F1", "=SUM(Items[Total])")
	on error message number code
		set the clipboard to saved
		error message number code
	end try
	set the clipboard to saved
	tell application "Microsoft Excel"
		set total of list object "Items" of ws to true
		-- Excel labels the totals row in the language of its menus; a
		-- fixed label makes the same workbook in every language.
		set value of range "A5" of ws to "Sum"
		select range "A1" of ws
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
end run
