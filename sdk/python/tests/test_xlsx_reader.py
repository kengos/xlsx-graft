from __future__ import annotations

import datetime
import threading
from pathlib import Path

import pytest

import workbook
from xlsx_graft import Book, CellError, ClosedError, Result, Row, SheetInfo, XlsxReader
from xlsx_graft.library import Library

DATA = (
    '<row r="1"><c r="A1" t="s"><v>1</v></c><c r="C1"><v>3</v></c>'
    '<c r="D1" t="b"><v>1</v></c></row>'
    '<row r="3"><c r="B3" s="1"><v>45322</v></c><c r="C3" s="2"><v>0.75</v></c>'
    '<c r="D3" s="3"><v>45322.000005787037</v></c><c r="E3" s="4"><v>1.5</v></c>'
    '<c r="F3" t="e"><v>#N/A</v></c><c r="G3" t="inlineStr"><is><t></t></is></c></row>'
    '<row r="4"/>'
)
BYTES = workbook.xlsx(
    workbook.worksheet("Data", DATA),
    workbook.chartsheet("Chart"),
    workbook.worksheet("Hidden", state="veryHidden"),
)


def rows_of(data: bytes, which: str | int = 0) -> tuple[Result[None], list[Row]]:
    with XlsxReader.open_bytes(data) as book:
        rows = list(book.sheet(which).rows())
    return book.result, rows


class TestOpenBytes:
    def test_gives_each_row_with_its_number_and_its_values_from_column_a(self) -> None:
        result, rows = rows_of(BYTES, "data")
        assert result == Result(success=True, value=None, diagnostics=(), error=None)
        assert [row.number for row in rows] == [1, 3, 4]
        assert rows[0].values == ("b", None, 3.0, True)
        assert rows[1].values == (
            None,
            datetime.date(2024, 1, 31),
            datetime.time(18, 0),
            datetime.datetime(2024, 1, 31, 0, 0, 0, 500_000),
            datetime.timedelta(hours=36),
            CellError("#N/A"),
            "",
        )
        assert rows[2].values == ()
        assert rows[1].values[3].tzinfo is None

    def test_lists_the_sheets(self) -> None:
        with XlsxReader.open_bytes(BYTES) as book:
            sheets = book.sheets()
        assert sheets == [
            SheetInfo(0, "Data", "worksheet", "visible"),
            SheetInfo(1, "Chart", "chartsheet", "visible"),
            SheetInfo(2, "Hidden", "worksheet", "veryHidden"),
        ]

    def test_reads_a_sheet_again_from_its_start(self) -> None:
        with XlsxReader.open_bytes(BYTES) as book:
            sheet = book.sheet("Data")
            first = next(sheet.rows())
            numbers = [first.number] + [row.number for row in sheet.rows()]
        assert numbers == [1, 1, 3, 4]
        assert book.result.success

    def test_copies_the_bytes_when_called(self) -> None:
        data = bytearray(BYTES)
        reader = XlsxReader.open_bytes(data)
        data[:] = b"not a zip"
        with reader as book:
            assert len(book.sheets()) == 3

    def test_raises_type_error_for_what_is_not_bytes_like(self) -> None:
        with pytest.raises(TypeError):
            XlsxReader.open_bytes("text")  # type: ignore[arg-type]


class TestRefusedSheet:
    @pytest.mark.parametrize(
        ("which", "code", "message"),
        [
            ("Missing", "sheet_not_found", 'no sheet is named "Missing"'),
            (
                9,
                "sheet_not_found",
                "there is no sheet at index 9: the workbook has 3, from index 0",
            ),
            ("Chart", "sheet_not_worksheet", 'sheet "Chart" is a chartsheet, which holds no cells'),
            (1, "sheet_not_worksheet", 'sheet "Chart" is a chartsheet, which holds no cells'),
        ],
    )
    def test_fails_reading_no_rows(self, which: str | int, code: str, message: str) -> None:
        result, rows = rows_of(BYTES, which)
        assert (result.success, result.value, rows) == (False, None, [])
        assert result.error is not None
        assert (result.error.code, result.error.severity, result.error.message) == (
            code,
            "error",
            message,
        )

    def test_goes_on_with_the_block_and_keeps_the_refusal_as_the_error(self) -> None:
        with XlsxReader.open_bytes(BYTES) as book:
            assert list(book.sheet("Missing").rows()) == []
            assert len(list(book.sheet("Data").rows())) == 3
        assert book.result.error is not None
        assert book.result.error.code == "sheet_not_found"

    def test_fails_with_sheet_unreadable_when_the_sheets_part_is_not_known(self) -> None:
        result, _ = rows_of(workbook.xlsx(workbook.unrelated("Lost")), "Lost")
        assert result.error is not None
        assert result.error.code == "sheet_unreadable"
        assert result.error.message == 'sheet "Lost" cannot be read; the diagnostics say why'
        assert result.diagnostics

    def test_fails_with_sheet_ambiguous_when_two_sheets_have_the_name(self) -> None:
        twice = workbook.xlsx(workbook.worksheet("Data"), workbook.worksheet("DATA"))
        result, _ = rows_of(twice, "data")
        assert result.error is not None
        assert result.error.message == 'more than one sheet is named "data", letter case aside'
        assert "sheet_name_duplicate" in [d.code for d in result.diagnostics]

    def test_refuses_a_sheet_name_that_is_not_utf_8(self) -> None:
        result, _ = rows_of(BYTES, "\udcff")
        assert result.error is not None
        assert result.error.code == "invalid_arg"

    def test_fails_with_workbook_unreadable_listing_no_sheets_when_the_bytes_are_no_workbook(
        self,
    ) -> None:
        with XlsxReader.open_bytes(b"not a zip") as book:
            assert book.sheets() == []
            assert list(book.sheet().rows()) == []
        assert book.result.error is not None
        assert book.result.error.code == "workbook_unreadable"
        assert book.result.error.message.startswith("the workbook cannot be read")
        assert book.result.diagnostics


def test_fails_without_an_error_when_a_problem_stops_the_reading() -> None:
    twice = '<row r="1"><c r="A1"><v>1</v></c></row><row r="1"/>'
    result, rows = rows_of(workbook.xlsx(workbook.worksheet("Data", twice)))
    assert [row.number for row in rows] == [1]
    assert (result.success, result.value, result.error) == (False, None, None)
    first = result.diagnostics[0]
    assert (first.code, first.severity) == ("cell_order_invalid", "error")
    assert first.location["part"].endswith("sheet1.xml")


def test_closes_the_book_when_the_block_raises_and_raises_on() -> None:
    reader = XlsxReader.open_bytes(BYTES)
    with pytest.raises(RuntimeError, match="boom"), reader as book:
        raise RuntimeError("boom")
    assert book.result.success
    with pytest.raises(ClosedError, match="not open"):
        book.sheets()
    with pytest.raises(ClosedError):
        book.sheet("Data")


def test_raises_closed_error_for_a_sheet_read_after_the_block() -> None:
    with XlsxReader.open_bytes(BYTES) as book:
        sheet = book.sheet()
        rows = sheet.rows()
        assert next(rows).number == 1
    with pytest.raises(ClosedError):
        next(sheet.rows())
    with pytest.raises(ClosedError):
        next(rows)


def test_closes_the_rows_a_generator_left_open() -> None:
    with XlsxReader.open_bytes(BYTES) as book:
        rows = book.sheet().rows()
        assert next(rows).number == 1
    assert book.result.success
    rows.close()


class TestMisuse:
    def test_raises_closed_error_before_the_block(self) -> None:
        book = XlsxReader.open_bytes(BYTES)
        with pytest.raises(ClosedError):
            book.sheets()
        with pytest.raises(ValueError, match="once the book is closed"):
            _ = book.result

    def test_raises_value_error_for_a_book_entered_twice(self) -> None:
        book = XlsxReader.open_bytes(BYTES)
        with book:
            pass
        with pytest.raises(ValueError, match="opened once"), book:
            pass

    @pytest.mark.parametrize("which", [b"Data", 1.0, True, None])
    def test_raises_type_error_for_a_sheet_neither_a_name_nor_an_index(self, which: object) -> None:
        with XlsxReader.open_bytes(BYTES) as book, pytest.raises(TypeError, match="not"):
            book.sheet(which)  # type: ignore[arg-type]

    @pytest.mark.parametrize("which", [-1, 2**64])
    def test_raises_value_error_for_an_index_out_of_range(self, which: int) -> None:
        with XlsxReader.open_bytes(BYTES) as book, pytest.raises(ValueError, match=str(which)):
            book.sheet(which)


class TestOpen:
    @pytest.mark.parametrize("kind", [str, Path, lambda path: str(path).encode()])
    def test_reads_the_workbook_at_a_path(self, tmp_path: Path, kind: object) -> None:
        path = tmp_path / "book.xlsx"
        path.write_bytes(BYTES)
        with XlsxReader.open(kind(path)) as book:  # type: ignore[operator]
            count = len(list(book.sheet().rows()))
        assert (count, book.result.success) == (3, True)

    def test_fails_with_io_the_hosts_reason_when_the_file_cannot_be_opened(self) -> None:
        with XlsxReader.open("/nowhere/book.xlsx") as book:
            assert book.sheets() == []
        assert (book.result.success, book.result.diagnostics) == (False, ())
        assert book.result.error is not None
        assert book.result.error.code == "io"
        assert "nowhere" in book.result.error.message

    def test_fails_with_invalid_arg_for_a_path_that_is_not_utf_8(self) -> None:
        for path in ("/tmp/\udcff.xlsx", b"/tmp/\xff.xlsx"):
            with XlsxReader.open(path) as book:
                pass
            assert book.result.error is not None
            assert book.result.error.code == "invalid_arg"

    def test_passes_library_on_to_library_open(
        self, monkeypatch: pytest.MonkeyPatch, engine_library: str
    ) -> None:
        paths: list[str | None] = []
        original = Library.open

        def spy(*, path: str | None = None) -> Library:
            paths.append(path)
            return original(path=engine_library)

        monkeypatch.setattr(Library, "open", spy)
        assert isinstance(XlsxReader.open("/nowhere.xlsx", library="/elsewhere.so"), Book)
        XlsxReader.open_bytes(b"", library="/also.so")
        assert paths == ["/elsewhere.so", "/also.so"]


class TestTurns:
    def test_two_generators_read_in_turns(self) -> None:
        with XlsxReader.open_bytes(BYTES) as book:
            first, second = book.sheet().rows(), book.sheet("Data").rows()
            pairs = [(a.values, b.values) for a, b in zip(first, second, strict=True)]
        assert all(a == b for a, b in pairs)
        assert len(pairs) == 3

    def test_threads_read_one_book_at_once(self) -> None:
        counts: list[int] = []
        with XlsxReader.open_bytes(BYTES) as book:

            def count() -> None:
                for _ in range(20):
                    counts.append(sum(1 for _row in book.sheet().rows()))

            threads = [threading.Thread(target=count) for _ in range(4)]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
        assert counts == [3] * 80
        assert book.result.success
