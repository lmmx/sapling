use facet::Facet;
use std::collections::HashMap;

/// Tree-sitter grammar representation
#[derive(Debug, Clone, Facet)]
pub struct Grammar {
    pub name: String,
    pub rules: HashMap<String, Rule>,
    pub extras: Vec<String>,
    pub externals: Vec<String>,
    pub inline: Vec<String>,
    pub supertypes: Vec<String>,
    pub word: Option<String>,
}

#[derive(Debug, Clone, Facet)]
#[repr(u8)]
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
    Alias {
        value: String,
        named: bool,
        content: Box<Rule>,
    },
}

/// Parse grammar from JSON using facet-json
pub fn parse_grammar(json: &str) -> Result<Grammar, GrammarError> {
    let grammar: Grammar =
        facet_json::from_str(json).map_err(|e| GrammarError::JsonParse(e.to_string()))?;

    Ok(grammar)
}

fn parse_rule(rule: &Rule) -> Result<Rule, GrammarError> {
    let rule_type = rule.r#type;

    match rule_type {
        "BLANK" => Ok(Rule::Blank),

        "STRING" => Ok(Rule::String { value: rule.value }),

        "PATTERN" => Ok(Rule::Pattern { value: rule.value }),

        "SYMBOL" => Ok(Rule::Symbol { name: rule.name }),

        "CHOICE" => Ok(Rule::Choice {
            members: rule.members,
        }),

        "SEQ" => Ok(Rule::Seq {
            members: rule.members,
        }),

        "REPEAT" => Ok(Rule::Repeat {
            content: rule.content,
        }),

        "REPEAT1" => Ok(Rule::Repeat1 {
            content: rule.content,
        }),

        "PREC" | "PREC_LEFT" | "PREC_RIGHT" => match rule_type {
            "PREC" => Ok(Rule::Prec {
                value: rule.value,
                content: rule.content,
            }),
            "PREC_LEFT" => Ok(Rule::PrecLeft {
                value: rule.value,
                content: rule.content,
            }),
            "PREC_RIGHT" => Ok(Rule::PrecRight {
                value: rule.value,
                content: rule.content,
            }),
            _ => unreachable!(),
        },

        "FIELD" => Ok(Rule::Field {
            name: rule.name,
            content: rule.content,
        }),

        "ALIAS" => Ok(Rule::Alias {
            value: rule.value,
            named: rule.named,
            content: rule.content,
        }),

        _ => Err(GrammarError::UnknownRuleType(rule_type)),
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
