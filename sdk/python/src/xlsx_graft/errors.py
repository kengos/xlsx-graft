"""What this package raises.

A call into the engine never raises; it returns a :class:`~xlsx_graft.Result`.
What raises is a setup that cannot reach the engine, a policy the engine does
not take (given to :class:`~xlsx_graft.Verifier` when it is made), or a book
used outside its ``with`` block. A call made wrongly raises Python's own ``TypeError`` or
``ValueError``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from xlsx_graft.diagnostic import Diagnostic


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


class PolicyInvalidError(Error, ValueError):
    """The engine does not take the policy given to
    :class:`~xlsx_graft.Verifier`. A ``ValueError`` too: the caller's policy
    is a value that is wrong.

    ``diagnostics`` says what is wrong, a ``policy_invalid`` diagnostic for
    each key. It is also raised when the check of the policy itself failed
    (``panic``, ``null_arg``, ``unknown_status``,
    ``diagnostics_unreadable``): the first of
    ``diagnostics`` is then that error, before any ``policy_invalid``
    diagnostic.
    """

    def __init__(self, diagnostics: tuple[Diagnostic, ...]) -> None:
        messages = "; ".join(diagnostic.message for diagnostic in diagnostics)
        super().__init__(f"the policy is not valid: {messages}")
        self.diagnostics = diagnostics
