//! Validation routines for Tree-sitter grammars.
//!
//! This module performs structural checks over parsed [`Grammar`](crate::grammar::Grammar)
//! definitions, such as verifying symbol references, ensuring all rules are reachable,
//! detecting left recursion, and confirming precedence consistency. It is used by
//! the `sapling` CLI and internal compiler passes to catch errors early.

use crate::grammar::{Grammar, Rule, RuleType};
use std::collections::{HashMap, HashSet};

/// Represents a validation failure encountered when checking a grammar.
///
/// Validation errors indicate issues such as undefined symbols, unreachable
/// rules, or recursive constructs that violate Tree-sitter's grammar constraints.
pub struct ValidationError {
    /// The descriptive human-readable error message.
    pub message: String,
}

impl ValidationError {
    /// Creates a new [`ValidationError`] from a message string.
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

/// Performs semantic validation of a parsed [`Grammar`](crate::grammar::Grammar).
///
/// This function runs several consistency passes over the grammar:
///
/// - Checks that all referenced symbols are defined.
/// - Warns about unreachable rules.
/// - Detects immediate left recursion.
/// - Verifies precedence consistency.
///
/// # Errors
///
/// Returns a [`ValidationError`] if any structural rule violation is detected.
pub fn validate(grammar: &Grammar) -> Result<(), ValidationError> {
    // Check for undefined symbol references
    check_undefined_symbols(grammar)?;

    // Check for unreachable rules
    check_unreachable_rules(grammar)?;

    // Detect problematic left recursion
    check_left_recursion(grammar);

    // Validate precedence usage
    check_precedence(grammar);

    Ok(())
}

fn check_undefined_symbols(grammar: &Grammar) -> Result<(), ValidationError> {
    let defined: HashSet<_> = grammar.rules.keys().collect();

    for (rule_name, rule) in &grammar.rules {
        check_rule_symbols(rule, &defined, rule_name)?;
    }

    Ok(())
}

fn check_rule_symbols(
    rule: &Rule,
    defined: &HashSet<&String>,
    context: &str,
) -> Result<(), ValidationError> {
    match rule.rule_type {
        RuleType::Symbol => {
            if let Some(name) = &rule.name {
                if !defined.contains(name) {
                    return Err(ValidationError::new(format!(
                        "undefined symbol '{name}' referenced in rule '{context}'"
                    )));
                }
            }
        }

        RuleType::Choice | RuleType::Seq => {
            if let Some(members) = &rule.members {
                for member in members {
                    check_rule_symbols(member, defined, context)?;
                }
            }
        }

        RuleType::Repeat
        | RuleType::Repeat1
        | RuleType::Prec
        | RuleType::PrecLeft
        | RuleType::PrecRight
        | RuleType::Field
        | RuleType::Alias => {
            if let Some(content) = &rule.content {
                check_rule_symbols(content, defined, context)?;
            }
        }

        RuleType::Blank
        | RuleType::String
        | RuleType::Pattern
        | RuleType::Token
        | RuleType::ImmediateToken
        | RuleType::Reserved
        | RuleType::PrecDynamic => {
            // terminals / others: nothing to traverse
        }
    }
    Ok(())
}

fn check_unreachable_rules(grammar: &Grammar) -> Result<(), ValidationError> {
    // Start from the first rule (convention: entry point)
    let entry_point = grammar
        .rules
        .keys()
        .next()
        .ok_or_else(|| ValidationError::new("grammar has no rules"))?;

    let mut reachable = HashSet::new();
    let mut to_visit = vec![entry_point.clone()];

    while let Some(rule_name) = to_visit.pop() {
        if !reachable.insert(rule_name.clone()) {
            continue; // Already visited
        }

        if let Some(rule) = grammar.rules.get(&rule_name) {
            collect_referenced_symbols(rule, &mut to_visit);
        }
    }

    for rule_name in grammar.rules.keys() {
        // grammar.inline is Option<Vec<String>> in grammar.rs: handle safely
        let inline_contains = grammar
            .inline
            .as_ref()
            .is_some_and(|v| v.contains(rule_name));

        if !reachable.contains(rule_name) && !inline_contains {
            eprintln!("warning: unreachable rule '{rule_name}'");
        }
    }

    Ok(())
}

fn collect_referenced_symbols(rule: &Rule, symbols: &mut Vec<String>) {
    match rule.rule_type {
        RuleType::Symbol => {
            if let Some(name) = &rule.name {
                symbols.push(name.clone());
            }
        }

        RuleType::Choice | RuleType::Seq => {
            if let Some(members) = &rule.members {
                for member in members {
                    collect_referenced_symbols(member, symbols);
                }
            }
        }

        RuleType::Repeat
        | RuleType::Repeat1
        | RuleType::Prec
        | RuleType::PrecLeft
        | RuleType::PrecRight
        | RuleType::Field
        | RuleType::Alias => {
            if let Some(content) = &rule.content {
                collect_referenced_symbols(content, symbols);
            }
        }

        RuleType::Blank
        | RuleType::String
        | RuleType::Pattern
        | RuleType::Token
        | RuleType::ImmediateToken
        | RuleType::Reserved
        | RuleType::PrecDynamic => {
            // nothing to collect
        }
    }
}

fn check_left_recursion(grammar: &Grammar) {
    // Detect immediate left recursion that lalrpop can't handle
    // lalrpop handles left recursion just fine, but we document it

    for (rule_name, rule) in &grammar.rules {
        if has_immediate_left_recursion(rule, rule_name) {
            // This is actually fine for lalrpop, just document it
            eprintln!("info: rule '{rule_name}' has left recursion (handled by lalrpop)");
        }
    }
}

fn has_immediate_left_recursion(rule: &Rule, target: &str) -> bool {
    match rule.rule_type {
        RuleType::Symbol => {
            if let Some(name) = &rule.name {
                return name == target;
            }
            false
        }

        RuleType::Seq => {
            if let Some(members) = &rule.members {
                members
                    .first()
                    .is_some_and(|first| has_immediate_left_recursion(first, target))
            } else {
                false
            }
        }

        RuleType::Choice => {
            if let Some(members) = &rule.members {
                members
                    .iter()
                    .any(|member| has_immediate_left_recursion(member, target))
            } else {
                false
            }
        }

        RuleType::Prec
        | RuleType::PrecLeft
        | RuleType::PrecRight
        | RuleType::Field
        | RuleType::Alias => {
            if let Some(content) = &rule.content {
                has_immediate_left_recursion(content, target)
            } else {
                false
            }
        }

        _ => false,
    }
}

fn check_precedence(grammar: &Grammar) {
    // Validate that precedence is used consistently
    let mut prec_levels: HashMap<String, Vec<i32>> = HashMap::new();

    for (rule_name, rule) in &grammar.rules {
        collect_precedence_levels(rule, &mut prec_levels, rule_name);
    }

    // Check for conflicting precedence declarations
    for (rule, levels) in &prec_levels {
        if levels.len() > 1 {
            eprintln!("warning: rule '{rule}' has multiple precedence levels: {levels:?}");
        }
    }
}

fn collect_precedence_levels(rule: &Rule, levels: &mut HashMap<String, Vec<i32>>, context: &str) {
    match rule.rule_type {
        RuleType::Prec | RuleType::PrecLeft | RuleType::PrecRight | RuleType::PrecDynamic => {
            // Use the helper method in grammar.rs if present, else read value via rule.value
            if let Some(p) = rule.precedence() {
                levels.entry(context.to_string()).or_default().push(p);
            }
            if let Some(content) = &rule.content {
                collect_precedence_levels(content, levels, context);
            }
        }

        RuleType::Choice | RuleType::Seq => {
            if let Some(members) = &rule.members {
                for member in members {
                    collect_precedence_levels(member, levels, context);
                }
            }
        }

        RuleType::Repeat | RuleType::Repeat1 | RuleType::Field | RuleType::Alias => {
            if let Some(content) = &rule.content {
                collect_precedence_levels(content, levels, context);
            }
        }

        _ => {}
    }
}
