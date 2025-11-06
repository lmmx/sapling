use crate::grammar::{Grammar, Rule};
use std::collections::{HashMap, HashSet};

pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

pub fn validate(grammar: &Grammar) -> Result<(), ValidationError> {
    // Check for undefined symbol references
    check_undefined_symbols(grammar)?;

    // Check for unreachable rules
    check_unreachable_rules(grammar)?;

    // Detect problematic left recursion
    check_left_recursion(grammar)?;

    // Validate precedence usage
    check_precedence(grammar)?;

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
    match rule {
        Rule::Symbol { name } => {
            if !defined.contains(name) {
                return Err(ValidationError::new(format!(
                    "undefined symbol '{}' referenced in rule '{}'",
                    name, context
                )));
            }
        }
        Rule::Choice { members } | Rule::Seq { members } => {
            for member in members {
                check_rule_symbols(member, defined, context)?;
            }
        }
        Rule::Repeat { content }
        | Rule::Repeat1 { content }
        | Rule::Prec { content, .. }
        | Rule::PrecLeft { content, .. }
        | Rule::PrecRight { content, .. }
        | Rule::Field { content, .. }
        | Rule::Alias { content, .. } => {
            check_rule_symbols(content, defined, context)?;
        }
        Rule::Blank | Rule::String { .. } | Rule::Pattern { .. } => {}
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
        if !reachable.contains(rule_name) && !grammar.inline.contains(rule_name) {
            eprintln!("warning: unreachable rule '{}'", rule_name);
        }
    }

    Ok(())
}

fn collect_referenced_symbols(rule: &Rule, symbols: &mut Vec<String>) {
    match rule {
        Rule::Symbol { name } => symbols.push(name.clone()),
        Rule::Choice { members } | Rule::Seq { members } => {
            for member in members {
                collect_referenced_symbols(member, symbols);
            }
        }
        Rule::Repeat { content }
        | Rule::Repeat1 { content }
        | Rule::Prec { content, .. }
        | Rule::PrecLeft { content, .. }
        | Rule::PrecRight { content, .. }
        | Rule::Field { content, .. }
        | Rule::Alias { content, .. } => {
            collect_referenced_symbols(content, symbols);
        }
        Rule::Blank | Rule::String { .. } | Rule::Pattern { .. } => {}
    }
}

fn check_left_recursion(grammar: &Grammar) -> Result<(), ValidationError> {
    // Detect immediate left recursion that lalrpop can't handle
    // lalrpop handles left recursion just fine, but we document it

    for (rule_name, rule) in &grammar.rules {
        if has_immediate_left_recursion(rule, rule_name) {
            // This is actually fine for lalrpop, just document it
            eprintln!(
                "info: rule '{}' has left recursion (handled by lalrpop)",
                rule_name
            );
        }
    }

    Ok(())
}

fn has_immediate_left_recursion(rule: &Rule, target: &str) -> bool {
    match rule {
        Rule::Symbol { name } => name == target,
        Rule::Seq { members } => members
            .first()
            .map(|first| has_immediate_left_recursion(first, target))
            .unwrap_or(false),
        Rule::Choice { members } => members
            .iter()
            .any(|member| has_immediate_left_recursion(member, target)),
        Rule::Prec { content, .. }
        | Rule::PrecLeft { content, .. }
        | Rule::PrecRight { content, .. }
        | Rule::Field { content, .. }
        | Rule::Alias { content, .. } => has_immediate_left_recursion(content, target),
        _ => false,
    }
}

fn check_precedence(grammar: &Grammar) -> Result<(), ValidationError> {
    // Validate that precedence is used consistently
    let mut prec_levels: HashMap<String, Vec<i32>> = HashMap::new();

    for (rule_name, rule) in &grammar.rules {
        collect_precedence_levels(rule, &mut prec_levels, rule_name);
    }

    // Check for conflicting precedence declarations
    for (rule, levels) in &prec_levels {
        if levels.len() > 1 {
            eprintln!(
                "warning: rule '{}' has multiple precedence levels: {:?}",
                rule, levels
            );
        }
    }

    Ok(())
}

fn collect_precedence_levels(rule: &Rule, levels: &mut HashMap<String, Vec<i32>>, context: &str) {
    match rule {
        Rule::Prec { value, content }
        | Rule::PrecLeft { value, content }
        | Rule::PrecRight { value, content } => {
            levels.entry(context.to_string()).or_default().push(*value);
            collect_precedence_levels(content, levels, context);
        }
        Rule::Choice { members } | Rule::Seq { members } => {
            for member in members {
                collect_precedence_levels(member, levels, context);
            }
        }
        Rule::Repeat { content }
        | Rule::Repeat1 { content }
        | Rule::Field { content, .. }
        | Rule::Alias { content, .. } => {
            collect_precedence_levels(content, levels, context);
        }
        _ => {}
    }
}
