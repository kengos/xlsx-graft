"""The package's own version. It tracks the engine workspace's version
(engine/Cargo.toml) while everything is pre-1.0.

A module of its own, under a private name: ``xlsx_graft.version`` is the call,
and hatchling reads the number from this file when it builds the wheel.
"""

__version__ = "0.0.0"
