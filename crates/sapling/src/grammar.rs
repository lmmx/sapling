//! Core structures and parsing logic for Tree-sitter grammars.
//!
//! This module defines the internal representation of a grammar as parsed from
//! Tree-sitter's JSON format. It uses [`facet_json`] for deserialization and
//! provides ergonomic accessors for inspecting rule properties and structure.

use facet::Facet;
use std::collections::HashMap;

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
/// See <https://tree-sitter.github.io/tree-sitter/assets/schemas/grammar.schema.json>
#[derive(Debug, Clone, Facet)]
pub struct Grammar {
    /// Optional `$schema` field from the JSON, typically used for schema
    /// validation or editor integration.
    #[facet(rename = "$schema")]
    pub schema: Option<String>,

    /// The short name of the grammar (e.g. `"javascript"` or `"rust"`).
    pub name: String,

    /// Optional name of a base grammar that this one inherits from.
    pub inherits: Option<String>,

    /// Map of all rule identifiers to their corresponding definitions.
    pub rules: HashMap<String, Rule>,

    /// “Extras” that may appear between other tokens, such as whitespace or comments.
    pub extras: Option<Vec<Rule>>,

    /// Rules implemented externally via a scanner.
    pub externals: Option<Vec<Rule>>,

    /// Names of rules that should be inlined into other rules.
    pub inline: Option<Vec<String>>,

    /// Precedence declarations that control operator binding order.
    pub precedences: Option<Vec<Vec<Precedence>>>,

    /// Explicit conflict groups expected during parsing.
    pub conflicts: Option<Vec<Vec<String>>>,

    /// Context-specific reserved word definitions.
    pub reserved: Option<HashMap<String, Vec<Rule>>>,

    /// The special rule name used to identify word tokens (keywords, identifiers, etc.).
    pub word: Option<String>,

    /// A list of node supertypes, grouping related syntactic forms.
    pub supertypes: Option<Vec<String>>,
}

/// A single precedence entry, either a named symbol or a literal string value.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum Precedence {
    /// A literal precedence string.
    String(String),

    /// A symbolic precedence name.
    Symbol {
        /// The identifier of the referenced symbol.
        name: String,
    },
}

/// Represents a grammar rule in the Tree-sitter format.
///
/// Each rule corresponds to a node in the grammar's rule graph, identified by a
/// [`RuleType`] and containing type-specific fields such as `members` or
/// `content`.
///
/// A `Rule` can be atomic (like a literal or regex) or composite
/// (like a sequence, choice, or precedence group). Together, they
/// form a self-describing syntax graph.
#[derive(Debug, Clone, Facet)]
pub struct Rule {
    /// The discriminant identifying what kind of rule this is.
    #[facet(rename = "type")]
    pub rule_type: RuleType,

    /// Optional literal or numeric value, depending on rule kind.
    pub value: Option<RuleValue>,

    /// Optional name used by `SYMBOL`, `FIELD`, or `ALIAS` rules.
    pub name: Option<String>,

    /// Optional nested rule for unary constructs such as `REPEAT` or `PREC`.
    pub content: Option<Box<Rule>>,

    /// Optional list of child rules for compound constructs (`SEQ`, `CHOICE`, etc.).
    pub members: Option<Vec<Rule>>,

    /// Whether the node produced by this rule is named.
    pub named: Option<bool>,

    /// Internal or generator-specific modifier flags.
    pub flags: Option<String>,

    /// Optional context label used for reserved-word handling.
    pub context_name: Option<String>,
}

/// A literal or numeric value attached to a rule node.
///
/// `RuleValue` abstracts small scalar payloads that alter how a rule behaves,
/// such as precedence numbers or literal match text.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum RuleValue {
    /// A string literal value (e.g. `"+"`, `"if"`).
    String(String),

    /// An integer numeric value (used by precedence modifiers).
    Integer(i32),
}

/// The enumeration of all recognized Tree-sitter rule types.
///
/// Each variant corresponds to one of the `type` strings found in the JSON
/// grammar format. Each variant captures a syntactic combinator, a primitive operation that
/// are composed to define language structure, the atoms of a grammar.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum RuleType {
    /// An empty (ε) production.
    #[facet(rename = "BLANK")]
    Blank,
    /// A literal string token.
    #[facet(rename = "STRING")]
    String,
    /// A regular-expression pattern token.
    #[facet(rename = "PATTERN")]
    Pattern,
    /// A reference to another named rule.
    #[facet(rename = "SYMBOL")]
    Symbol,
    /// A rule that matches one of several alternatives.
    #[facet(rename = "CHOICE")]
    Choice,
    /// A sequential composition of member rules.
    #[facet(rename = "SEQ")]
    Seq,
    /// A zero-or-more repetition of a rule.
    #[facet(rename = "REPEAT")]
    Repeat,
    /// A one-or-more repetition of a rule.
    #[facet(rename = "REPEAT1")]
    Repeat1,
    /// A generic precedence wrapper.
    #[facet(rename = "PREC")]
    Prec,
    /// A left-associative precedence wrapper.
    #[facet(rename = "PREC_LEFT")]
    PrecLeft,
    /// A right-associative precedence wrapper.
    #[facet(rename = "PREC_RIGHT")]
    PrecRight,
    /// A dynamic (runtime) precedence wrapper.
    #[facet(rename = "PREC_DYNAMIC")]
    PrecDynamic,
    /// A named field applied to a subrule.
    #[facet(rename = "FIELD")]
    Field,
    /// An alias providing an alternate node name.
    #[facet(rename = "ALIAS")]
    Alias,
    /// A tokenization wrapper.
    #[facet(rename = "TOKEN")]
    Token,
    /// A token that must appear immediately without leading trivia.
    #[facet(rename = "IMMEDIATE_TOKEN")]
    ImmediateToken,
    /// A reserved internal placeholder.
    #[facet(rename = "RESERVED")]
    Reserved,
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

impl Rule {
    /// Returns the canonical string name of this rule type.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self.rule_type {
            RuleType::Blank => "BLANK",
            RuleType::String => "STRING",
            RuleType::Pattern => "PATTERN",
            RuleType::Symbol => "SYMBOL",
            RuleType::Choice => "CHOICE",
            RuleType::Seq => "SEQ",
            RuleType::Repeat => "REPEAT",
            RuleType::Repeat1 => "REPEAT1",
            RuleType::Prec => "PREC",
            RuleType::PrecLeft => "PREC_LEFT",
            RuleType::PrecRight => "PREC_RIGHT",
            RuleType::PrecDynamic => "PREC_DYNAMIC",
            RuleType::Field => "FIELD",
            RuleType::Alias => "ALIAS",
            RuleType::Token => "TOKEN",
            RuleType::ImmediateToken => "IMMEDIATE_TOKEN",
            RuleType::Reserved => "RESERVED",
        }
    }

    /// Returns `true` if this rule represents a terminal (lexical) token.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self.rule_type, RuleType::String | RuleType::Pattern)
    }

    /// Returns `true` if this rule is a symbol reference.
    #[must_use]
    pub fn is_symbol(&self) -> bool {
        matches!(self.rule_type, RuleType::Symbol)
    }

    /// Returns the referenced symbol name, if applicable.
    #[must_use]
    pub fn symbol_name(&self) -> Option<&str> {
        if self.is_symbol() {
            self.name.as_deref()
        } else {
            None
        }
    }

    /// Returns the numeric precedence value if this rule is a precedence wrapper.
    #[must_use]
    pub fn precedence(&self) -> Option<i32> {
        match self.rule_type {
            RuleType::Prec | RuleType::PrecLeft | RuleType::PrecRight | RuleType::PrecDynamic => {
                self.value.as_ref().and_then(|v| match v {
                    RuleValue::Integer(i) => Some(*i),
                    RuleValue::String(_) => None,
                })
            }
            _ => None,
        }
    }

    /// Returns the literal string value if this is a `STRING` rule.
    #[must_use]
    pub fn string_value(&self) -> Option<&str> {
        if matches!(self.rule_type, RuleType::String) {
            self.value.as_ref().and_then(|v| match v {
                RuleValue::String(s) => Some(s.as_str()),
                RuleValue::Integer(_) => None,
            })
        } else {
            None
        }
    }

    /// Returns the pattern source if this is a `PATTERN` rule.
    #[must_use]
    pub fn pattern_value(&self) -> Option<&str> {
        if matches!(self.rule_type, RuleType::Pattern) {
            self.value.as_ref().and_then(|v| match v {
                RuleValue::String(s) => Some(s.as_str()),
                RuleValue::Integer(_) => None,
            })
        } else {
            None
        }
    }
}

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

        let grammar = parse_grammar(json).unwrap();
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

        let grammar = parse_grammar(json).unwrap();
        let expr_rule = grammar.rules.get("expr").unwrap();
        assert_eq!(expr_rule.precedence(), Some(1));
        assert!(matches!(expr_rule.rule_type, RuleType::PrecLeft));
    }
}
