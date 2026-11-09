-- Makes fixtures/chart.xlsx in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/chart.applescript <output .xlsx path>
--
-- The output path must be one Excel may write without asking, such as a
-- folder under ~/Library/Containers/com.microsoft.Excel/Data/, and no file
-- may be there yet: Excel would ask whether to replace it.

on run argv
	set outPath to item 1 of argv
	tell application "Microsoft Excel"
		set wb to make new workbook
		set ws to worksheet 1 of wb
		set name of ws to "Sheet1"
		set value of range "A1:B4" of ws to {{"Item", "Qty"}, {"A", 2}, {"B", 3}, {"C", 5}}
		set co to make new chart object at ws with properties {left position:200, top:10, width:360, height:220}
		set ch to chart of co
		set chart type of ch to column clustered
		set source data ch source range "A1:B4" of ws
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
end run
