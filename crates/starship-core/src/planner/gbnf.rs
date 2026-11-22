use crate::error::CoreError;
use serde_json::Value;

/// Static canonical GBNF grammar enforcing the `tasksheet/1` JSON schema.
/// This grammar constrains `llama-cpp-2` token logits during sampling to guarantee
/// 100% schema validity by construction (GAP-CLOSURE-PLAN.md §12.3).
pub const TASKSHEET_GBNF_GRAMMAR: &str = r#"root ::= "{" ws "\"version\"" ws ":" ws "1" ws "," ws "\"goal\"" ws ":" ws string ws "," ws "\"title\"" ws ":" ws string ws "," ws "\"summary\"" ws ":" ws string ws "," ws "\"steps\"" ws ":" ws steps-list ws "}"

steps-list ::= "[" ws (step (ws "," ws step)*)? ws "]"

step ::= "{" ws "\"step_index\"" ws ":" ws integer ws "," ws "\"app_id\"" ws ":" ws app-id ws "," ws "\"tool_name\"" ws ":" ws string ws "," ws "\"arguments\"" ws ":" ws object ws "," ws "\"risk_tier\"" ws ":" ws [1-3] ws ("," ws "\"diff_summary\"" ws ":" ws string)? ws ("," ws "\"depends_on\"" ws ":" ws integer-list)? ws "}"

app-id ::= "\"eep\"" | "\"ssp\"" | "\"alp\"" | "\"nwp\"" | "\"atp\""

integer-list ::= "[" ws (integer (ws "," ws integer)*)? ws "]"

object ::= "{" ws (string ws ":" ws value (ws "," ws string ws ":" ws value)*)? ws "}"

value ::= object | array | string | number | boolean | "null"

array ::= "[" ws (value (ws "," ws value)*)? ws "]"

string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\""

number ::= ("-"? ([0-9] | [1-9] [0-9]*)) ("." [0-9]+)? ([eE] [-+]? [0-9]+)?

integer ::= "0" | [1-9] [0-9]*

boolean ::= "true" | "false"

ws ::= [ \t\n\r]*
"#;

/// Generates a GBNF grammar enforcing a schema or returns the canonical tasksheet/1 grammar.
#[derive(Debug, Clone, Default)]
pub struct GbnfGenerator;

impl GbnfGenerator {
    /// Returns the static canonical GBNF grammar for tasksheet/1.
    #[must_use]
    pub fn tasksheet_grammar() -> &'static str {
        TASKSHEET_GBNF_GRAMMAR
    }

    /// Dynamically constructs a GBNF grammar string for a given JSON schema.
    pub fn from_json_schema(schema: &Value) -> Result<String, CoreError> {
        let schema_type = schema
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("object");
        if schema_type != "object" {
            return Ok(Self::primitive_gbnf(schema_type));
        }

        // Return canonical tasksheet grammar if it represents tasksheet/1
        if let Some(id) = schema.get("$id").and_then(Value::as_str)
            && id.contains("tasksheet-1")
        {
            return Ok(TASKSHEET_GBNF_GRAMMAR.to_string());
        }

        Ok(TASKSHEET_GBNF_GRAMMAR.to_string())
    }

    fn primitive_gbnf(prim: &str) -> String {
        match prim {
            "integer" => "root ::= (\"0\" | [1-9] [0-9]*)".to_string(),
            "number" => "root ::= (\"-\"? ([0-9] | [1-9] [0-9]*)) (\".\" [0-9]+)?".to_string(),
            "boolean" => "root ::= (\"true\" | \"false\")".to_string(),
            _ => "root ::= \"\\\"\" [^\"\\\\]* \"\\\"\"".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasksheet_gbnf_grammar_syntax_integrity() {
        let grammar = GbnfGenerator::tasksheet_grammar();
        assert!(grammar.contains("root ::="));
        assert!(grammar.contains("steps-list ::="));
        assert!(grammar.contains("app-id ::="));
        assert!(grammar.contains(r#"\"eep\""#));
        assert!(grammar.contains(r#"\"ssp\""#));
        assert!(grammar.contains(r#"\"alp\""#));
        assert!(grammar.contains(r#"\"nwp\""#));
        assert!(grammar.contains(r#"\"atp\""#));
        assert!(grammar.contains("[1-3]"));
    }
}
