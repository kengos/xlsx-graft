"""What this package raises.

A call into the engine never raises; it returns a :class:`~xlsx_graft.Result`.
What raises is a setup that cannot reach the engine, or a book used outside
its ``with`` block. A call made wrongly raises Python's own ``TypeError`` or
``ValueError``.
"""


class Error(Exception):
    """The root of what this package raises."""


class LibraryNotFoundError(Error):
    """No engine library could be found or loaded.

    A configuration mistake, not a failed call, so it is raised rather than
    returned as a :class:`~xlsx_graft.Result`.
    """


class ClosedError(Error, ValueError):
    """A :class:`~xlsx_graft.Book` or its sheets used outside the book's
    ``with`` block: before it opened the book, or after it closed it. A
    ``ValueError`` too, as an I/O operation on a closed file is.
    """
