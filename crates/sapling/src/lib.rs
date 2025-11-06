//! A Rust-native tree-sitter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::multiple_crate_versions)]

pub mod grammar;
pub mod validate;

pub use grammar::{parse_grammar, Grammar, GrammarError, Rule};
pub use validate::{validate, ValidationError};
