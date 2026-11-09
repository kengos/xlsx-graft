-- Makes fixtures/date1904.xlsx in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/date1904.applescript <output .xlsx path>
--
-- The output path must be one Excel may write without asking, such as a
-- folder under ~/Library/Containers/com.microsoft.Excel/Data/, and no file
-- may be there yet: Excel would ask whether to replace it.
--
-- The workbook uses the 1904 date system: serial 0 is 1904-01-01, and
-- 2024-01-15 is serial 43844 (45306 in the 1900 date system).

on run argv
	set outPath to item 1 of argv
	tell application "Microsoft Excel"
		set wb to make new workbook
		set date 1904 of wb to true
		set ws to worksheet 1 of wb
		set name of ws to "Sheet1"
		set value of range "A1:B1" of ws to {{"Serial", "Date"}}
		set value of range "A2:A4" of ws to {{0}, {43844}, {1.5}}
		set formula of range "B2" of ws to "=A2"
		set formula of range "B3" of ws to "=A3"
		set formula of range "B4" of ws to "=A4"
		set number format of range "B2:B3" of ws to "yyyy-mm-dd"
		set number format of range "B4" of ws to "yyyy-mm-dd hh:mm"
		set formula of range "C2" of ws to "=DATE(2024,1,15)"
		set number format of range "C2" of ws to "yyyy-mm-dd"
		save workbook as wb filename outPath file format Excel XML file format
		close active workbook saving no
	end tell
end run
