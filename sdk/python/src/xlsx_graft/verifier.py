"""Verifies workbooks through the C ABI, under the host's policy and the
rules of the caller.
"""

from __future__ import annotations

import os
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from xlsx_graft import status_diagnostic, verify_report
from xlsx_graft.errors import PolicyInvalidError
from xlsx_graft.library import Library
from xlsx_graft.policy import policy_json, rules_json
from xlsx_graft.result import Result
from xlsx_graft.verify_call import VerifyCall

# The most JSON the engine takes as the policy and the rules together.
MAX_OPTIONS_BYTES = 2 << 20


class Verifier:
    """Verifies workbooks under the host's policy (given to the constructor,
    which has the engine check it) and the rules of the caller (given to each
    call).

    A verifier does not change once it is made, so one may be shared by any
    number of threads, and their calls may run at once.

    Example::

        import sys

        verifier = xlsx_graft.Verifier(policy={"max_formula_bytes": 32768})
        result = verifier.verify("book.xlsx", targets=["excel >= 2016"], allow_parts=["macro"])
        for d in result.diagnostics:
            print(f"{d.severity} {d.code}: {d.message}", file=sys.stderr)
        if result.error:
            print(result.error.message, file=sys.stderr)
    """

    def __init__(
        self, policy: Mapping[str, Any] | None = None, *, library: str | None = None
    ) -> None:
        """``policy`` is the host's policy: the engine's limits, ``fail_on``,
        ``deny_scheduled`` and ``forbid`` (``{"functions": [...], "parts":
        [...]}``). Its keys are snake_case or camelCase, those of the dicts
        inside it too, and its values are not changed. ``None`` is the
        engine's defaults. The policy is copied as JSON now: changing
        ``policy`` after changes nothing.

        ``library`` is the engine library's path, used when
        ``XLSX_GRAFT_LIBRARY`` is not set.

        Raises :class:`~xlsx_graft.PolicyInvalidError` when the engine does
        not take the policy, :class:`~xlsx_graft.LibraryNotFoundError` when
        no engine library is found, ``TypeError`` for a policy that is not a
        mapping or holds a value JSON cannot write, ``ValueError`` for a
        NaN or an infinity in it, and ``RecursionError`` for dicts and lists
        nested past Python's recursion limit.
        """
        self._call = VerifyCall(Library.open(path=library))
        self._policy = "null" if policy is None else self._checked(policy_json(policy))

    def verify(
        self,
        path: str | bytes | os.PathLike[str] | os.PathLike[bytes],
        *,
        targets: Sequence[str] | None = None,
        best_effort: Sequence[str] | None = None,
        allow_functions: Sequence[str] | None = None,
        allow_parts: Sequence[str] | None = None,
        fail_on: str | None = None,
    ) -> Result[None]:
        """Verifies the workbook at ``path``.

        ``targets`` are the versions the workbook must run in
        (``"excel >= 2016"``); a problem with one is an error.
        ``best_effort`` are the versions it should run in; a problem with one
        is a warning. ``allow_functions`` are functions the engine refuses by default
        that are allowed (named without an ``_xlfn.`` prefix).
        ``allow_parts`` are kinds of part the engine refuses by default that
        are allowed (``macro``,
        ``external_link``, ``dde``, ``ole``, ``activex``, ``data_connection``,
        ``web_publish``). ``fail_on`` is ``"warn"`` to fail on a warning too;
        the stricter of this and the policy's ``fail_on`` counts.

        The result is successful when the engine's ``ok`` is true, and it has
        no value. A file that cannot be read is an ``io`` ``error``. A fault
        of the rules is a ``rules_invalid`` or ``target_invalid`` diagnostic,
        and the workbook is not opened. With ``fail_on="warn"`` a result
        fails on a warning with no error diagnostic. For a path that is not
        UTF-8 this package reads the file, so an unreadable file is an
        ``io`` ``error`` and the rules are not checked.

        Raises ``TypeError`` for a path of the wrong kind or a rule that JSON
        cannot write, and ``ValueError`` for a NaN or an infinity in a rule or
        for a policy and rules that make more than 2 MiB of JSON. A rule nested
        past Python's recursion limit raises ``RecursionError``.
        """
        options = self._options(targets, best_effort, allow_functions, allow_parts, fail_on)
        encoded = os.fsencode(path)
        try:
            encoded.decode("utf-8")
        except UnicodeDecodeError:
            return self._verify_file(encoded, options)
        return verify_report.call(*self._call.verify_path(encoded, options))

    def verify_bytes(
        self,
        data: bytes | bytearray | memoryview,
        *,
        targets: Sequence[str] | None = None,
        best_effort: Sequence[str] | None = None,
        allow_functions: Sequence[str] | None = None,
        allow_parts: Sequence[str] | None = None,
        fail_on: str | None = None,
    ) -> Result[None]:
        """As :meth:`verify`, the workbook being the bytes of ``data``,
        copied now: changing ``data`` during the call changes nothing.

        Raises ``TypeError`` for what is not bytes-like (anything
        ``memoryview`` takes is read), and as :meth:`verify`.
        """
        options = self._options(targets, best_effort, allow_functions, allow_parts, fail_on)
        copy = memoryview(data).tobytes()
        return verify_report.call(*self._call.verify_bytes(copy, options))

    # Returns the policy's JSON when the engine accepts it; raises
    # PolicyInvalidError otherwise.
    def _checked(self, policy: str) -> str:
        result = verify_report.call(*self._call.check_policy(policy.encode()))
        if result.failed:
            error = () if result.error is None else (result.error,)
            raise PolicyInvalidError((*error, *result.diagnostics))
        return policy

    # The options JSON as UTF-8: the policy and the rules the caller gave, by
    # the engine's names.
    def _options(
        self,
        targets: Sequence[str] | None,
        best_effort: Sequence[str] | None,
        allow_functions: Sequence[str] | None,
        allow_parts: Sequence[str] | None,
        fail_on: str | None,
    ) -> bytes:
        rules: dict[str, Any] = {
            "targets": targets,
            "bestEffort": best_effort,
            "functions": None if allow_functions is None else {"allow": allow_functions},
            "parts": None if allow_parts is None else {"allow": allow_parts},
            "failOn": fail_on,
        }
        given = {key: value for key, value in rules.items() if value is not None}
        options = f'{{"policy":{self._policy},"rules":{rules_json(given)}}}'.encode()
        if len(options) > MAX_OPTIONS_BYTES:
            message = (
                f"the policy and the rules are {len(options)} bytes of JSON; "
                f"the engine takes {MAX_OPTIONS_BYTES}"
            )
            raise ValueError(message)
        return options

    # The engine takes no path that is not UTF-8, so this package reads the
    # file and verifies its bytes; a file that cannot be read is an ``io``
    # failure.
    def _verify_file(self, path: bytes, options: bytes) -> Result[None]:
        try:
            data = Path(os.fsdecode(path)).read_bytes()
        except OSError as error:
            message = f"the file could not be read: {error}"
            return Result.failure((), status_diagnostic.of("io", message))
        return verify_report.call(*self._call.verify_bytes(data, options))
