//! Core types for representing Tree-sitter grammar rules.
//!
//! This module contains the types used to model grammar rules and their
//! structure according to the Tree-sitter JSON schema.

use facet::Facet;

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
    #[facet(default)]
    pub value: Option<RuleValue>,

    /// Optional name used by `SYMBOL`, `FIELD`, or `ALIAS` rules.
    #[facet(default)]
    pub name: Option<String>,

    /// Optional nested rule for unary constructs such as `REPEAT` or `PREC`.
    #[facet(default)]
    pub content: Option<Box<Rule>>,

    /// List of child rules for compound constructs (`SEQ`, `CHOICE`, etc.).
    #[facet(default)]
    pub members: Vec<Rule>,

    /// Whether the node produced by this rule is named.
    #[facet(default)]
    pub named: Option<bool>,

    /// Internal or generator-specific modifier flags.
    #[facet(default)]
    pub flags: Option<String>,

    /// Optional context label used for reserved-word handling.
    #[facet(default)]
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
    fn test_parse_simple_rule() {
        let json = r#"{
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
        }"#;

        let rules: Vec<Rule> = facet_json::from_str(&json).unwrap_or_else(|e| {
            eprintln!("JSON parse error:\n{}", e);
            std::process::exit(1);
        });

        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_parse_precedence() {
        let json = r#"{
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
        }"#;

        let rule: Rule = facet_json::from_str(&json).unwrap_or_else(|e| {
            eprintln!("JSON parse error:\n{}", e);
            std::process::exit(1);
        });
        assert_eq!(rule.precedence(), Some(1));
        assert!(matches!(rule.rule_type, RuleType::PrecLeft));
    }
}
