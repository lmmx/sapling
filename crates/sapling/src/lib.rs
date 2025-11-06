//! A Rust-native tree-sitter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::multiple_crate_versions)]

/// Core structures and parsing logic for Tree-sitter grammars.
///
/// This module defines how Sapling understands and manipulates the
/// declarative shape of a language: the grammar itself. Everything else
/// in the compiler builds upon these types.
pub mod grammar;

/// Grammar validation and consistency checking utilities.
///
/// Validation exists to protect downstream stages (like codegen and analysis)
/// from malformed grammars. It enforces Tree-sitter's invariants and ensures
/// that what's parsed is also semantically meaningful.
pub mod validate;

pub use grammar::{parse_grammar, Grammar, GrammarError, Rule};
pub use validate::{validate, ValidationError};
