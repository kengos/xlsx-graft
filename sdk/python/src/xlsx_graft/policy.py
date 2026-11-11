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
    JSON cannot write, ``ValueError`` for a NaN or an infinity, and
    ``RecursionError`` for dicts and lists nested past Python's recursion
    limit.
    """
    if not isinstance(policy, Mapping):
        message = f"policy: is a mapping or None, not {type(policy).__name__}"
        raise TypeError(message)
    return rules_json(_camelized(policy))


def rules_json(value: Any) -> str:
    """``value`` as compact JSON, with no NaN or infinity (``ValueError``)."""
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
