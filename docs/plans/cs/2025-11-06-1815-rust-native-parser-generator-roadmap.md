# Concrete Implementation Options

## What NOT to do (no detail):
- syn + proc_macro codegen (compile time explosion)
- Runtime table interpreter like rowan (performance penalty)
- PEG parsers (can't handle left recursion natively)
- pest (runtime PEG, same issues)
- hand-coded recursive descent (reinventing lalrpop)

## Viable paths ranked:

### 1. **lalrpop generation from tree-sitter grammar JSON** ⭐ RECOMMENDED
- Parse grammar.json → Generate .lalrpop file → lalrpop compiles to Rust
- Zero runtime cost, pure Rust code generation
- Native precedence/associativity support
- Handles left recursion perfectly

### 2. **lrpar + lrtable with runtime loading**
- Parse grammar.json → Generate parse tables → Load at runtime
- More flexible but slower than lalrpop
- Still pure Rust, just runtime dispatch

### 3. **chumsky (parser combinators)**
- Write grammar as Rust code directly
- No JSON parsing needed
- Performance close to hand-written but verbose

---

# Implementation Plan: Tree-Sitter → lalrpop Code Generator

## Architecture

```
grammar.json → Rust structs → Validation → lalrpop IR → .lalrpop file
                ↓
         (using facet-json)
```

**Three phases:**
1. **Parse & represent**: JSON → typed Rust grammar structures
2. **Validate & lower**: Check validity, resolve conflicts, convert to lalrpop concepts
3. **Emit**: Generate .lalrpop text

## Key Design Decisions

### Why lalrpop?

1. **Parsing algorithm match**: Tree-sitter uses GLR, lalrpop uses LR(1). Most tree-sitter grammars don't actually need GLR's full power - they work fine with LR(1). We detect conflicts and provide clear error messages.

2. **Zero runtime overhead**: Generates pure Rust at build time. No table interpreter, no indirection.

3. **Native precedence**: lalrpop's `#[precedence]` annotations map directly to tree-sitter's `prec`, `prec_left`, `prec_right`.

4. **External scanner pattern**: Tree-sitter's external scanners become Rust traits that the generated parser calls.

5. **Battle-tested**: Powers real production parsers, well-maintained.

### Why facet-json?

You already use it in textum (cli.rs:46). Tree-sitter grammar JSON is simple - facet handles it fine. Consistency with existing codebase.

### External scanners strategy

Tree-sitter external scanners (for context-sensitive parsing like Python indentation) become:

```rust
pub trait ExternalScanner {
    fn scan(&mut self, lexer: &Lexer, valid_symbols: &[bool]) -> Option<Symbol>;
}
```

Generated parser calls `scanner.scan()` at designated points. Users implement the trait.

### Incremental parsing

**Punted to v2**. Tree-sitter's killer feature is incremental reparsing, but:
- Requires persistent parse trees (rowan-style green trees)
- Edit tracking and reuse detection
- Too complex for initial implementation

Full reparses with lalrpop are fast enough for most use cases. Add incrementality later if needed.

---

# Giacometti Dev Journal: 2025-11-06 Tree-Sitter Rust Native

## Current State
- No implementation exists
- Analyzed tree-sitter-c2rust codebase (parser.rs is core: 4.3k lines)
- Identified lalrpop as generation target

## Missing Components
- Grammar JSON parser (facet-json based, not serde)
- Grammar validation (undefined symbols, precedence conflicts, LR(1) compatibility check)
- lalrpop IR representation (nonterminals, terminals, productions with precedence)
- lalrpop code emitter (generates .lalrpop grammar files from IR)
- External scanner trait (replaces C ABI with Rust traits)
- Build integration (build.rs to run conversion)
- Test suite (validate against tree-sitter reference grammars)

## Key Insights from parser.rs Analysis
- Core parsing is in `ts_parser__advance()` - LR shift/reduce dispatch
- Precedence handled in `ts_parser__compare_versions()` and action selection
- Stack operations in `ts_stack_*` functions
- Subtree building via `ts_subtree_new_node()`
- External scanners called via function pointers in C - will become trait calls

## Divergence from rust-sitter
- rust-sitter: grammar.json → tree-sitter CLI → C code → cc compiles → FFI
- Our approach: grammar.json → Rust codegen → .lalrpop → rustc → no FFI

## Next Steps
1. Define grammar JSON → Rust struct mappings (using facet-json)
2. Implement grammar validator (detect LR conflicts)
3. Build lalrpop IR representation
4. Write .lalrpop emitter
5. Create external scanner trait definition

---

# Initial Implementation

I'll start with the core data structures and JSON parsing, based on analyzing parser.rs:```rust
// tree-sitter-rust-native/src/grammar.rs
use std::collections::HashMap;

/// Tree-sitter grammar representation
#[derive(Debug, Clone)]
pub struct Grammar {
    pub name: String,
    pub rules: HashMap<String, Rule>,
    pub extras: Vec<String>,
    pub externals: Vec<String>,
    pub inline: Vec<String>,
    pub supertypes: Vec<String>,
    pub word: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Rule {
    /// Empty/epsilon production
    Blank,
    /// Literal string match
    String { value: String },
    /// Regex pattern match
    Pattern { value: String },
    /// Reference to another rule
    Symbol { name: String },
    /// Choice between alternatives (A | B)
    Choice { members: Vec<Rule> },
    /// Sequence (A B C)
    Seq { members: Vec<Rule> },
    /// Zero or more (A*)
    Repeat { content: Box<Rule> },
    /// One or more (A+)
    Repeat1 { content: Box<Rule> },
    /// Precedence (no associativity)
    Prec { value: i32, content: Box<Rule> },
    /// Left associative precedence
    PrecLeft { value: i32, content: Box<Rule> },
    /// Right associative precedence
    PrecRight { value: i32, content: Box<Rule> },
    /// Named field in AST
    Field { name: String, content: Box<Rule> },
    /// Alias (renames symbol in parse tree)
    Alias { value: String, named: bool, content: Box<Rule> },
}

/// Parse grammar from JSON using facet-json
pub fn parse_grammar(json: &str) -> Result<Grammar, GrammarError> {
    let value: facet_json::Value = facet_json::from_str(json)
        .map_err(|e| GrammarError::JsonParse(e.to_string()))?;

    let obj = value.as_object()
        .ok_or_else(|| GrammarError::InvalidStructure("expected object at root".into()))?;

    let name = obj.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GrammarError::MissingField("name"))?
        .to_string();

    let rules_obj = obj.get("rules")
        .and_then(|v| v.as_object())
        .ok_or_else(|| GrammarError::MissingField("rules"))?;

    let mut rules = HashMap::new();
    for (rule_name, rule_value) in rules_obj {
        rules.insert(rule_name.clone(), parse_rule(rule_value)?);
    }

    let extras = parse_string_array(obj.get("extras"))?;
    let externals = parse_string_array(obj.get("externals"))?;
    let inline = parse_string_array(obj.get("inline"))?;
    let supertypes = parse_string_array(obj.get("supertypes"))?;
    let word = obj.get("word").and_then(|v| v.as_str()).map(String::from);

    Ok(Grammar {
        name,
        rules,
        extras,
        externals,
        inline,
        supertypes,
        word,
    })
}

fn parse_rule(value: &facet_json::Value) -> Result<Rule, GrammarError> {
    let obj = value.as_object()
        .ok_or_else(|| GrammarError::InvalidStructure("rule must be object".into()))?;

    let rule_type = obj.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GrammarError::MissingField("type"))?;

    match rule_type {
        "BLANK" => Ok(Rule::Blank),

        "STRING" => {
            let value = obj.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| GrammarError::MissingField("value"))?
                .to_string();
            Ok(Rule::String { value })
        }

        "PATTERN" => {
            let value = obj.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| GrammarError::MissingField("value"))?
                .to_string();
            Ok(Rule::Pattern { value })
        }

        "SYMBOL" => {
            let name = obj.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| GrammarError::MissingField("name"))?
                .to_string();
            Ok(Rule::Symbol { name })
        }

        "CHOICE" => {
            let members_arr = obj.get("members")
                .and_then(|v| v.as_array())
                .ok_or_else(|| GrammarError::MissingField("members"))?;
            let mut members = Vec::new();
            for member in members_arr {
                members.push(parse_rule(member)?);
            }
            Ok(Rule::Choice { members })
        }

        "SEQ" => {
            let members_arr = obj.get("members")
                .and_then(|v| v.as_array())
                .ok_or_else(|| GrammarError::MissingField("members"))?;
            let mut members = Vec::new();
            for member in members_arr {
                members.push(parse_rule(member)?);
            }
            Ok(Rule::Seq { members })
        }

        "REPEAT" => {
            let content = obj.get("content")
                .ok_or_else(|| GrammarError::MissingField("content"))?;
            Ok(Rule::Repeat {
                content: Box::new(parse_rule(content)?),
            })
        }

        "REPEAT1" => {
            let content = obj.get("content")
                .ok_or_else(|| GrammarError::MissingField("content"))?;
            Ok(Rule::Repeat1 {
                content: Box::new(parse_rule(content)?),
            })
        }

        "PREC" | "PREC_LEFT" | "PREC_RIGHT" => {
            let value = obj.get("value")
                .and_then(|v| v.as_number())
                .and_then(|n| n.as_i64())
                .ok_or_else(|| GrammarError::MissingField("value"))? as i32;

            let content = obj.get("content")
                .ok_or_else(|| GrammarError::MissingField("content"))?;
            let content = Box::new(parse_rule(content)?);

            match rule_type {
                "PREC" => Ok(Rule::Prec { value, content }),
                "PREC_LEFT" => Ok(Rule::PrecLeft { value, content }),
                "PREC_RIGHT" => Ok(Rule::PrecRight { value, content }),
                _ => unreachable!(),
            }
        }

        "FIELD" => {
            let name = obj.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| GrammarError::MissingField("name"))?
                .to_string();
            let content = obj.get("content")
                .ok_or_else(|| GrammarError::MissingField("content"))?;
            Ok(Rule::Field {
                name,
                content: Box::new(parse_rule(content)?),
            })
        }

        "ALIAS" => {
            let value = obj.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| GrammarError::MissingField("value"))?
                .to_string();
            let named = obj.get("named")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let content = obj.get("content")
                .ok_or_else(|| GrammarError::MissingField("content"))?;
            Ok(Rule::Alias {
                value,
                named,
                content: Box::new(parse_rule(content)?),
            })
        }

        _ => Err(GrammarError::UnknownRuleType(rule_type.to_string())),
    }
}

fn parse_string_array(value: Option<&facet_json::Value>) -> Result<Vec<String>, GrammarError> {
    match value {
        None => Ok(Vec::new()),
        Some(v) => {
            let arr = v.as_array()
                .ok_or_else(|| GrammarError::InvalidStructure("expected array".into()))?;
            arr.iter()
                .map(|item| {
                    item.as_str()
                        .ok_or_else(|| GrammarError::InvalidStructure("expected string in array".into()))
                        .map(String::from)
                })
                .collect()
        }
    }
}

#[derive(Debug)]
pub enum GrammarError {
    JsonParse(String),
    MissingField(&'static str),
    InvalidStructure(String),
    UnknownRuleType(String),
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GrammarError::JsonParse(e) => write!(f, "JSON parse error: {}", e),
            GrammarError::MissingField(field) => write!(f, "missing field: {}", field),
            GrammarError::InvalidStructure(msg) => write!(f, "invalid structure: {}", msg),
            GrammarError::UnknownRuleType(ty) => write!(f, "unknown rule type: {}", ty),
        }
    }
}

impl std::error::Error for GrammarError {}
```

```rust
// tree-sitter-rust-native/src/validate.rs
use crate::grammar::{Grammar, Rule};
use std::collections::{HashMap, HashSet};

pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
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
    let entry_point = grammar.rules.keys().next()
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
            eprintln!("info: rule '{}' has left recursion (handled by lalrpop)", rule_name);
        }
    }

    Ok(())
}

fn has_immediate_left_recursion(rule: &Rule, target: &str) -> bool {
    match rule {
        Rule::Symbol { name } => name == target,
        Rule::Seq { members } => {
            members.first()
                .map(|first| has_immediate_left_recursion(first, target))
                .unwrap_or(false)
        }
        Rule::Choice { members } => {
            members.iter().any(|member| has_immediate_left_recursion(member, target))
        }
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
```

This establishes the foundation. The next step would be building the lalrpop IR and emitter, but this gives you the core grammar representation and validation logic based on analyzing parser.rs's precedence handling and stack operations.
