-- Makes fixtures/dynamic-array.xlsx in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/dynamic-array.applescript <output .xlsx path>
--
-- The output path must be one Excel may write without asking, such as a
-- folder under ~/Library/Containers/com.microsoft.Excel/Data/, and no file
-- may be there yet: Excel would ask whether to replace it.
--
-- Excel's AppleScript has no Formula2: a formula set through `formula` is
-- saved as a legacy formula, which does not spill. Each formula is pasted
-- from the clipboard instead, which Excel takes as typed, so it spills.
-- The clipboard's text is restored at the end.

on pasteFormula(ws, cellName, formulaText)
	tell application "Microsoft Excel" to select range cellName of ws
	set the clipboard to formulaText
	tell application "Microsoft Excel" to paste worksheet ws
end pasteFormula

on run argv
	set outPath to item 1 of argv
	set saved to the clipboard
	tell application "Microsoft Excel"
		activate
		set wb to make new workbook
		set ws to worksheet 1 of wb
		set name of ws to "Sheet1"
		set value of range "B1:B4" of ws to {{3}, {1}, {4}, {2}}
	end tell
	pasteFormula(ws, "A1", "=SEQUENCE(5)")
	pasteFormula(ws, "C1", "=SORT(B1:B4)")
	pasteFormula(ws, "D1", "=FILTER(B1:B4,B1:B4>2)")
	pasteFormula(ws, "E1", "=UNIQUE({1;1;2})")
	pasteFormula(ws, "F1", "=SUM(A1#)")
	set the clipboard to saved
	tell application "Microsoft Excel"
		select range "A1" of ws
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
end run
