//! Core structures and parsing logic for Tree-sitter grammars.
//!
//! This module defines the internal representation of a grammar as parsed from
//! Tree-sitter's JSON format. It uses [`facet_json`] for deserialization and
//! provides ergonomic accessors for inspecting rule properties and structure.

use facet::Facet;
use std::collections::HashMap;

pub mod rules;

pub use rules::{Rule, RuleType, RuleValue};

/// Represents a full Tree-sitter grammar definition.
///
/// This structure directly mirrors the serialized JSON format produced by
/// `tree-sitter generate --json`. It captures the complete rule set along with
/// auxiliary metadata such as precedences, conflicts, and supertypes.
///
/// `Grammar` is the root artifact in Sapling's parsing pipeline. It holds
/// both syntactic and semantic scaffolding (rules, precedence, conflicts,
/// and contextual hints) that together define a language's formal structure.
///
/// Only the `name` and `rules` fields are required.
///
/// See <https://tree-sitter.github.io/tree-sitter/assets/schemas/grammar.schema.json>
#[derive(Debug, Clone, Facet)]
pub struct Grammar {
    /// Optional `$schema` field from the JSON, typically used for schema
    /// validation or editor integration.
    #[facet(default, rename = "$schema")]
    pub schema: Option<String>,

    /// The short name of the grammar (e.g. `"javascript"` or `"rust"`).
    pub name: String, // required: no default null

    /// Optional name of a base grammar that this one inherits from.
    #[facet(default)]
    pub inherits: Option<String>,

    /// Map of all rule identifiers to their corresponding definitions.
    pub rules: HashMap<String, Rule>, // required: no default null

    /// “Extras” that may appear between other tokens, such as whitespace or comments.
    #[facet(default)]
    pub extras: Option<Vec<Rule>>,

    /// Precedence declarations that control operator binding order.
    #[facet(default)]
    pub precedences: Option<Vec<Vec<Precedence>>>,

    /// Context-specific reserved word definitions.
    #[facet(default)]
    pub reserved: Option<HashMap<String, Vec<Rule>>>,

    /// Rules implemented externally via a scanner.
    #[facet(default)]
    pub externals: Option<Vec<Rule>>,

    /// Names of rules that should be inlined into other rules.
    #[facet(default)]
    pub inline: Option<Vec<String>>,

    /// Explicit conflict groups expected during parsing.
    #[facet(default)]
    pub conflicts: Option<Vec<Vec<String>>>,

    /// The special rule name used to identify word tokens (keywords, identifiers, etc.).
    #[facet(default)]
    pub word: Option<String>,

    /// A list of node supertypes, grouping related syntactic forms.
    #[facet(default)]
    pub supertypes: Option<Vec<String>>,
}

/// A single precedence entry, either a named symbol or a literal string value.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum Precedence {
    /// A literal precedence string.
    String(String),

    /// A symbolic precedence reference.
    SymbolRule {
        /// The discriminant identifying this as a SYMBOL rule.
        /// This field will always be `RuleType::Symbol`
        #[facet(rename = "type")]
        rule_type: RuleType, // will always be RuleType::Symbol
        /// The identifier of the referenced symbol.
        name: String,
    },
}

/// Parse a JSON grammar definition into a strongly typed [`Grammar`] structure.
///
/// # Errors
///
/// Returns [`GrammarError::JsonParse`] if the provided string is not valid JSON
/// or fails schema deserialization.
pub fn parse_grammar(json: &str) -> Result<Grammar, GrammarError> {
    facet_json::from_str(json).map_err(|e| GrammarError::JsonParse(e.to_string()))
}

/// Possible errors raised during grammar parsing or validation.
#[derive(Debug)]
pub enum GrammarError {
    /// The input JSON was syntactically invalid or structurally mismatched.
    JsonParse(String),

    /// Higher-level structural or semantic validation failure.
    Validation(String),
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GrammarError::JsonParse(e) => write!(f, "JSON parse error: {e}"),
            GrammarError::Validation(msg) => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for GrammarError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_grammar() {
        let json = r#"{
            "name": "test",
            "rules": {
                "source_file": {
                    "type": "SYMBOL",
                    "name": "expression"
                },
                "expression": {
                    "type": "CHOICE",
                    "members": [
                        {
                            "type": "STRING",
                            "value": "hello"
                        },
                        {
                            "type": "PATTERN",
                            "value": "[0-9]+"
                        }
                    ]
                }
            }
        }"#;

        let grammar = parse_grammar(json).unwrap_or_else(|e| {
            if let GrammarError::JsonParse(inner) = e {
                eprintln!("JSON parse error:\n{}", inner);
            } else {
                eprintln!("Grammar error: {}", e);
            }
            std::process::exit(1);
        });

        assert_eq!(grammar.name, "test");
        assert_eq!(grammar.rules.len(), 2);
    }

    #[test]
    fn test_parse_precedence() {
        let json = r#"{
            "name": "test",
            "rules": {
                "expr": {
                    "type": "PREC_LEFT",
                    "value": 1,
                    "content": {
                        "type": "SEQ",
                        "members": [
                            {"type": "SYMBOL", "name": "expr"},
                            {"type": "STRING", "value": "+"},
                            {"type": "SYMBOL", "name": "expr"}
                        ]
                    }
                }
            }
        }"#;

        let grammar = parse_grammar(json).unwrap_or_else(|e| {
            if let GrammarError::JsonParse(inner) = e {
                eprintln!("JSON parse error:\n{}", inner);
            } else {
                eprintln!("Grammar error: {}", e);
            }
            std::process::exit(1);
        });
        let expr_rule = grammar.rules.get("expr").unwrap();
        assert_eq!(expr_rule.precedence(), Some(1));
        assert!(matches!(expr_rule.rule_type, RuleType::PrecLeft));
    }
}
