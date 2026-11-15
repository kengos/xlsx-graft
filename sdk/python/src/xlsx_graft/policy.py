"""Writes a policy, or the rules of a call, as the JSON the engine takes:
keys in camelCase.
"""

from __future__ import annotations

import json
import re
from collections.abc import Mapping
from typing import Any


def policy_json(policy: object) -> str:
    """The policy as JSON, the keys of its dicts (those inside its lists too)
    written in camelCase and the values left as they are (only dicts and
    lists are searched for keys: a tuple is not): ``max_entries``
    becomes ``maxEntries``, and a camelCase key stays.

    Raises ``TypeError`` for a policy that is not a mapping or holds a value
    JSON cannot write, ``ValueError`` for a NaN, an infinity, or a string (a
    key or a value) holding a lone surrogate, and ``RecursionError`` for
    dicts and lists nested past Python's recursion limit.
    """
    if not isinstance(policy, Mapping):
        message = f"policy: is a mapping or None, not {type(policy).__name__}"
        raise TypeError(message)
    return rules_json(_camelized(policy))


def rules_json(value: Any) -> str:
    """``value`` as compact JSON in ASCII (``\\uXXXX`` for each character outside ASCII).

    Raises ``ValueError`` for a NaN, an infinity, or a string (a key or a
    value) holding a lone surrogate, which UTF-8 cannot encode.
    """
    # The first call to json.dumps keeps non-ASCII characters so that encoding
    # the text as UTF-8 raises UnicodeEncodeError for a lone surrogate. The
    # second call writes the JSON sent to the engine: ASCII, with \uXXXX for
    # each character outside ASCII, and the 2 MiB limit counts those bytes.
    text = json.dumps(value, allow_nan=False, ensure_ascii=False, separators=(",", ":"))
    try:
        text.encode("utf-8")
    except UnicodeEncodeError as error:
        message = (
            f"a string of the policy or the rules holds a lone surrogate "
            f"(U+{ord(text[error.start]):04X}), which UTF-8 cannot encode"
        )
        raise ValueError(message) from None
    return json.dumps(value, allow_nan=False, separators=(",", ":"))


# Recurses as deep as the caller's dicts and lists nest, with no limit of its
# own: a nesting past Python's recursion limit raises RecursionError.
def _camelized(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {_camel(str(key)): _camelized(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_camelized(item) for item in value]
    return value


def _camel(key: str) -> str:
    return re.sub(r"_([a-z\d])", lambda match: match.group(1).upper(), key)
