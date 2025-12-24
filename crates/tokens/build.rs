use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TokensConfig {
    meta: Meta,
    tokens: Vec<Token>,
    semantic_tokens: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    name: String,
    file_extensions: Vec<String>,
    scope_name: String,
}

#[derive(Debug, Deserialize)]
struct Token {
    name: String,
    #[serde(default)]
    literal: Option<String>,
    #[serde(default)]
    regex: Option<String>,
    category: String,
    #[serde(default)]
    textmate_scope: Option<String>,
    #[serde(default)]
    priority: Option<u32>,
}

fn main() {
    println!("cargo::rerun-if-changed=tokens.toml");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let tokens_path = Path::new(&manifest_dir).join("tokens.toml");
    let tokens_content = fs::read_to_string(&tokens_path).expect("Failed to read tokens.toml");
    let config: TokensConfig =
        toml::from_str(&tokens_content).expect("Failed to parse tokens.toml");

    // Generate token_kind.rs
    let out_dir = env::var("OUT_DIR").unwrap();
    let token_kind_path = Path::new(&out_dir).join("token_kind.rs");
    let token_kind_code = generate_token_kind(&config);
    fs::write(&token_kind_path, token_kind_code).expect("Failed to write token_kind.rs");

    // Generate TextMate grammar
    let textmate_path = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("editors/vscode/syntaxes/toy.tmLanguage.json");
    fs::create_dir_all(textmate_path.parent().unwrap()).ok();
    let textmate_grammar = generate_textmate_grammar(&config);
    fs::write(&textmate_path, textmate_grammar).expect("Failed to write TextMate grammar");

    // Generate metadata module
    let metadata_path = Path::new(&out_dir).join("metadata.rs");
    let metadata_code = generate_metadata(&config);
    fs::write(&metadata_path, metadata_code).expect("Failed to write metadata.rs");
}

fn generate_token_kind(config: &TokensConfig) -> String {
    let mut code = String::new();

    code.push_str("use logos::Logos;\n\n");
    code.push_str("#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    code.push_str("pub enum TokenKind {\n");

    for token in &config.tokens {
        if let Some(literal) = &token.literal {
            let escaped = literal.replace('\\', "\\\\").replace('"', "\\\"");
            code.push_str(&format!("    #[token(\"{}\")]\n", escaped));
        } else if let Some(regex) = &token.regex {
            // Patterns with unbounded repetition need allow_greedy in logos 0.16+
            let is_greedy = regex.contains(".*")
                || regex.contains(".+")
                || regex.ends_with('*')
                || regex.ends_with('+');
            let priority = token.priority.unwrap_or(2);
            if is_greedy {
                code.push_str(&format!(
                    "    #[regex(r#\"{}\"#, priority = {}, allow_greedy = true)]\n",
                    regex, priority
                ));
            } else if token.priority.is_some() {
                code.push_str(&format!(
                    "    #[regex(r#\"{}\"#, priority = {})]\n",
                    regex, priority
                ));
            } else {
                code.push_str(&format!("    #[regex(r#\"{}\"#)]\n", regex));
            }
        }
        code.push_str(&format!("    {},\n\n", token.name));
    }

    code.push_str("}\n\n");

    // Generate category method
    code.push_str("impl TokenKind {\n");
    code.push_str("    pub fn category(self) -> &'static str {\n");
    code.push_str("        match self {\n");
    for token in &config.tokens {
        code.push_str(&format!(
            "            TokenKind::{} => \"{}\",\n",
            token.name, token.category
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Generate is_trivia method
    code.push_str("    pub fn is_trivia(self) -> bool {\n");
    code.push_str("        matches!(self, ");
    let trivia: Vec<_> = config
        .tokens
        .iter()
        .filter(|t| t.category == "trivia")
        .map(|t| format!("Self::{}", t.name))
        .collect();
    code.push_str(&trivia.join(" | "));
    code.push_str(")\n");
    code.push_str("    }\n\n");

    // Generate is_newline method
    code.push_str("    pub fn is_newline(self) -> bool {\n");
    code.push_str("        matches!(self, Self::NewLine)\n");
    code.push_str("    }\n");

    code.push_str("}\n\n");

    // Generate Display impl
    code.push_str("impl std::fmt::Display for TokenKind {\n");
    code.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    code.push_str("        match self {\n");
    for token in &config.tokens {
        let display = if let Some(lit) = &token.literal {
            lit.clone()
        } else {
            token.name.to_lowercase().replace('_', " ")
        };
        let escaped = display
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('{', "{{")
            .replace('}', "}}");
        code.push_str(&format!(
            "            TokenKind::{} => write!(f, \"{}\"),\n",
            token.name, escaped
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

fn generate_textmate_grammar(config: &TokensConfig) -> String {
    use serde_json::json;

    let patterns: Vec<_> = config
        .tokens
        .iter()
        .filter_map(|token| {
            let scope = token.textmate_scope.as_ref()?;
            let match_pattern = if let Some(literal) = &token.literal {
                regex_escape(literal)
            } else if let Some(regex) = &token.regex {
                regex.clone()
            } else {
                return None;
            };
            Some(json!({
                "name": scope,
                "match": match_pattern
            }))
        })
        .collect();

    let grammar = json!({
        "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
        "name": config.meta.name,
        "scopeName": config.meta.scope_name,
        "fileTypes": config.meta.file_extensions,
        "patterns": patterns
    });

    let mut result = serde_json::to_string_pretty(&grammar).unwrap();
    result.push('\n');
    result
}

fn generate_metadata(config: &TokensConfig) -> String {
    let mut code = String::new();

    // Generate semantic token type mapping (extends TokenKind impl from token_kind.rs)
    code.push_str("impl TokenKind {\n");
    code.push_str("    pub fn semantic_token_type(self) -> Option<&'static str> {\n");
    code.push_str("        let category = self.category();\n");
    code.push_str("        match category {\n");
    for (category, semantic_type) in &config.semantic_tokens {
        code.push_str(&format!(
            "            \"{}\" => Some(\"{}\"),\n",
            category, semantic_type
        ));
    }
    code.push_str("            _ => None,\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Generate textmate scope mapping
    code.push_str("    pub fn textmate_scope(self) -> Option<&'static str> {\n");
    code.push_str("        match self {\n");
    for token in &config.tokens {
        if let Some(scope) = &token.textmate_scope {
            code.push_str(&format!(
                "            TokenKind::{} => Some(\"{}\"),\n",
                token.name, scope
            ));
        }
    }
    code.push_str("            _ => None,\n");
    code.push_str("        }\n");
    code.push_str("    }\n");

    code.push_str("}\n");

    code
}

fn regex_escape(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}
