"""What this package raises.

A call into the engine never raises; it returns a :class:`~xlsx_graft.Result`.
What raises is a setup that cannot reach the engine.
"""


class Error(Exception):
    """The root of what this package raises."""


class LibraryNotFoundError(Error):
    """No engine library could be found or loaded.

    A configuration mistake, not a failed call, so it is raised rather than
    returned as a :class:`~xlsx_graft.Result`.
    """
