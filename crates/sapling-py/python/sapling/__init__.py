"""Python bindings for sapling - Rust-native tree-sitter."""

from __future__ import annotations

from ._sapling import (
    PyGrammar as Grammar,
)

__version__ = "0.1.0"

__all__ = [
    "Grammar",
]
