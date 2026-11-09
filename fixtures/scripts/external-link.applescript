-- Makes fixtures/external-link.xlsx and the workbook it links to,
-- fixtures/external-source.xlsx, in Excel for Mac (see fixtures/README.md).
--
-- osascript fixtures/scripts/external-link.applescript <output folder>
--
-- The folder must be one Excel may write without asking, such as a folder
-- under ~/Library/Containers/com.microsoft.Excel/Data/, and must not hold
-- either workbook yet. Both are saved in it, so the link's target is the
-- source's file name alone.

on run argv
	set outDir to item 1 of argv
	tell application "Microsoft Excel"
		set src to make new workbook
		set srcSheet to worksheet 1 of src
		set name of srcSheet to "Sheet1"
		set value of range "A1:B1" of srcSheet to {{42, "ext"}}
		save workbook as src filename (outDir & "/external-source.xlsx") file format Excel XML file format
		set lnk to make new workbook
		set ws to worksheet 1 of lnk
		set name of ws to "Sheet1"
		set formula of range "A1" of ws to "='[external-source.xlsx]Sheet1'!$A$1"
		set formula of range "B1" of ws to "='[external-source.xlsx]Sheet1'!$B$1"
		set formula of range "C1" of ws to "=SUM('[external-source.xlsx]Sheet1'!$A$1,1)"
		save workbook as lnk filename (outDir & "/external-link.xlsx") file format Excel XML file format
		close workbook "external-link.xlsx" saving no
		close workbook "external-source.xlsx" saving no
	end tell
end run
