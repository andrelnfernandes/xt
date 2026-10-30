//! Lexer, parser and evaluator for mini-SQL `WHERE` predicates over tabular rows.
//!
//! Designed for the Xylem table viewer:
//! - Pure safe Rust, zero dependencies.
//! - Bounded recursion and resource usage to prevent denial of service.
//! - Supports column identifiers by header name or spreadsheet letters (A, B, C...).
//! - Operators: comparison (=, !=, <>, <, <=, >, >=), logic (AND, OR, NOT),
//!   strings (LIKE, ILIKE), and set/null checks (IN, IS NULL, IS NOT NULL).

use std::fmt;

/// Values encountered in tabular cells or filter literals.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl Value {
    /// Parse a string slice into a typed Value (inferring int, float, bool or text).
    pub fn infer(s: &str) -> Self {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Value::Null;
        }
        if let Ok(b) = trimmed.parse::<bool>() {
            return Value::Bool(b);
        }
        if let Ok(i) = trimmed.parse::<i64>() {
            return Value::Int(i);
        }
        if let Ok(f) = trimmed.parse::<f64>()
            && f.is_finite()
        {
            return Value::Float(f);
        }
        Value::Text(trimmed.to_string())
    }

    /// Convert to string representation for pattern matching or display.
    #[allow(dead_code)]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s.as_str()),
            Value::Null => None,
            _ => None,
        }
    }

    /// Truthiness check for boolean contexts.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::Text(s) => !s.is_empty(),
            Value::Null => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Text(s) => write!(f, "{s}"),
        }
    }
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Like,
    ILike,
    And,
    Or,
}

/// Abstract Syntax Tree for SQL expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    /// Column reference by name (or letter like "A", "B").
    Column(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    IsNull(Box<Expr>),
    IsNotNull(Box<Expr>),
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    NumberLit(String),
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Comma,
    LParen,
    RParen,
    And,
    Or,
    Not,
    Like,
    ILike,
    In,
    Is,
    True,
    False,
    Null,
    Where,
}

struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(&(i, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
                continue;
            }

            match ch {
                '(' => {
                    self.chars.next();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.chars.next();
                    tokens.push(Token::RParen);
                }
                ',' => {
                    self.chars.next();
                    tokens.push(Token::Comma);
                }
                '=' => {
                    self.chars.next();
                    tokens.push(Token::Eq);
                }
                '!' => {
                    self.chars.next();
                    if let Some(&(_, '=')) = self.chars.peek() {
                        self.chars.next();
                        tokens.push(Token::NotEq);
                    } else {
                        return Err(format!("Unexpected character '!' at index {i}"));
                    }
                }
                '<' => {
                    self.chars.next();
                    if let Some(&(_, '=')) = self.chars.peek() {
                        self.chars.next();
                        tokens.push(Token::LtEq);
                    } else if let Some(&(_, '>')) = self.chars.peek() {
                        self.chars.next();
                        tokens.push(Token::NotEq);
                    } else {
                        tokens.push(Token::Lt);
                    }
                }
                '>' => {
                    self.chars.next();
                    if let Some(&(_, '=')) = self.chars.peek() {
                        self.chars.next();
                        tokens.push(Token::GtEq);
                    } else {
                        tokens.push(Token::Gt);
                    }
                }
                '\'' | '"' => {
                    let quote = ch;
                    self.chars.next();
                    let start = i + 1;
                    let mut end = start;
                    let mut escaped = false;
                    let mut closed = false;
                    for (j, c) in self.chars.by_ref() {
                        if c == quote && !escaped {
                            end = j;
                            closed = true;
                            break;
                        }
                        escaped = c == '\\' && !escaped;
                    }
                    if !closed {
                        return Err("Unterminated string literal".to_string());
                    }
                    tokens.push(Token::StringLit(
                        self.input[start..end]
                            .replace("\\'", "'")
                            .replace("\\\"", "\""),
                    ));
                }
                _ if ch.is_ascii_digit() || ch == '-' || ch == '+' => {
                    let start = i;
                    let mut has_dot = false;
                    self.chars.next();
                    while let Some(&(_j, c)) = self.chars.peek() {
                        if c.is_ascii_digit() {
                            self.chars.next();
                        } else if c == '.' && !has_dot {
                            has_dot = true;
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                    let s = &self.input[start
                        ..self
                            .chars
                            .peek()
                            .map(|&(j, _)| j)
                            .unwrap_or(self.input.len())];
                    tokens.push(Token::NumberLit(s.to_string()));
                }
                _ if ch.is_alphabetic() || ch == '_' => {
                    let start = i;
                    self.chars.next();
                    while let Some(&(_j, c)) = self.chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                    let s = &self.input[start
                        ..self
                            .chars
                            .peek()
                            .map(|&(j, _)| j)
                            .unwrap_or(self.input.len())];
                    let upper = s.to_ascii_uppercase();
                    let tok = match upper.as_str() {
                        "WHERE" => Token::Where,
                        "AND" => Token::And,
                        "OR" => Token::Or,
                        "NOT" => Token::Not,
                        "LIKE" => Token::Like,
                        "ILIKE" => Token::ILike,
                        "IN" => Token::In,
                        "IS" => Token::Is,
                        "NULL" => Token::Null,
                        "TRUE" => Token::True,
                        "FALSE" => Token::False,
                        _ => Token::Ident(s.to_string()),
                    };
                    tokens.push(tok);
                }
                _ => {
                    return Err(format!("Unexpected character {ch:?} at index {i}"));
                }
            }
        }
        Ok(tokens)
    }
}

/// Parser for filter expressions with operator precedence.
pub struct SqlFilter {
    tokens: Vec<Token>,
    pos: usize,
}

impl SqlFilter {
    /// Parse a query string like `WHERE age > 30 AND status = 'active'` or `age > 30`.
    pub fn parse(input: &str) -> Result<Expr, String> {
        if input.len() > 1024 {
            return Err("Query string too long (maximum 1024 bytes)".to_string());
        }
        let tokens = Lexer::new(input).tokenize()?;
        if tokens.is_empty() {
            return Err("Empty query".to_string());
        }
        if tokens.len() > 128 {
            return Err("Query too complex (maximum 128 tokens)".to_string());
        }
        let mut parser = SqlFilter { tokens, pos: 0 };
        // Optional leading WHERE keyword
        if parser.peek() == Some(&Token::Where) {
            parser.pos += 1;
        }
        let expr = parser.parse_or(0)?;
        if parser.pos < parser.tokens.len() {
            return Err(format!(
                "Unexpected extra token: {:?}",
                parser.tokens[parser.pos]
            ));
        }
        Ok(expr)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse_or(&mut self, depth: usize) -> Result<Expr, String> {
        if depth > 64 {
            return Err("Expression nesting too deep".to_string());
        }
        let mut left = self.parse_and(depth + 1)?;
        while self.peek() == Some(&Token::Or) {
            self.next();
            let right = self.parse_and(depth + 1)?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self, depth: usize) -> Result<Expr, String> {
        if depth > 64 {
            return Err("Expression nesting too deep".to_string());
        }
        let mut left = self.parse_not(depth + 1)?;
        while self.peek() == Some(&Token::And) {
            self.next();
            let right = self.parse_not(depth + 1)?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self, depth: usize) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Not) {
            self.next();
            let expr = self.parse_comparison(depth + 1)?;
            Ok(Expr::Not(Box::new(expr)))
        } else {
            self.parse_comparison(depth)
        }
    }

    fn parse_comparison(&mut self, depth: usize) -> Result<Expr, String> {
        let left = self.parse_primary(depth + 1)?;

        match self.peek() {
            Some(Token::Eq) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::NotEq) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::NotEq,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::Lt) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::Lt,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::LtEq) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::LtEq,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::Gt) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::GtEq) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::GtEq,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::Like) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::Like,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::ILike) => {
                self.next();
                let right = self.parse_primary(depth + 1)?;
                Ok(Expr::Binary {
                    op: BinaryOp::ILike,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::Is) => {
                self.next();
                let negated = if self.peek() == Some(&Token::Not) {
                    self.next();
                    true
                } else {
                    false
                };
                if self.peek() == Some(&Token::Null) {
                    self.next();
                    if negated {
                        Ok(Expr::IsNotNull(Box::new(left)))
                    } else {
                        Ok(Expr::IsNull(Box::new(left)))
                    }
                } else {
                    Err("Expected NULL after IS [NOT]".to_string())
                }
            }
            Some(Token::In) | Some(Token::Not) => {
                let negated = if self.peek() == Some(&Token::Not) {
                    self.next();
                    if self.peek() != Some(&Token::In) {
                        return Err("Expected IN after NOT".to_string());
                    }
                    self.next();
                    true
                } else {
                    self.next();
                    false
                };

                if self.next() != Some(Token::LParen) {
                    return Err("Expected '(' after IN".to_string());
                }
                let mut list = Vec::new();
                while self.peek() != Some(&Token::RParen) {
                    list.push(self.parse_primary(depth + 1)?);
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    } else {
                        break;
                    }
                }
                if self.next() != Some(Token::RParen) {
                    return Err("Expected ')' closing IN list".to_string());
                }
                Ok(Expr::InList {
                    expr: Box::new(left),
                    list,
                    negated,
                })
            }
            _ => Ok(left),
        }
    }

    fn parse_primary(&mut self, depth: usize) -> Result<Expr, String> {
        match self.next() {
            Some(Token::LParen) => {
                let expr = self.parse_or(depth + 1)?;
                if self.next() != Some(Token::RParen) {
                    return Err("Unclosed parenthesis".to_string());
                }
                Ok(expr)
            }
            Some(Token::Ident(name)) => Ok(Expr::Column(name)),
            Some(Token::StringLit(s)) => Ok(Expr::Literal(Value::Text(s))),
            Some(Token::NumberLit(n)) => {
                if let Ok(i) = n.parse::<i64>() {
                    Ok(Expr::Literal(Value::Int(i)))
                } else if let Ok(f) = n.parse::<f64>() {
                    Ok(Expr::Literal(Value::Float(f)))
                } else {
                    Err(format!("Invalid number literal: {n}"))
                }
            }
            Some(Token::Null) => Ok(Expr::Literal(Value::Null)),
            Some(Token::True) => Ok(Expr::Literal(Value::Bool(true))),
            Some(Token::False) => Ok(Expr::Literal(Value::Bool(false))),
            Some(other) => Err(format!("Unexpected token: {other:?}")),
            None => Err("Unexpected end of input".to_string()),
        }
    }
}

/// Helper to match SQL LIKE patterns (% for any string, _ for any single char).
pub fn sql_like(pattern: &str, text: &str, case_insensitive: bool) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = if case_insensitive {
        (
            pattern.to_lowercase().chars().collect(),
            text.to_lowercase().chars().collect(),
        )
    } else {
        (pattern.chars().collect(), text.chars().collect())
    };

    let (mut pi, mut ti) = (0, 0);
    let (mut star_idx, mut match_idx) = (None, 0);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '_' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '%' {
            star_idx = Some(pi);
            pi += 1;
            match_idx = ti;
        } else if let Some(si) = star_idx {
            pi = si + 1;
            match_idx += 1;
            ti = match_idx;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '%' {
        pi += 1;
    }

    pi == p.len()
}

/// Context providing access to row values by column name or 0-indexed column letter.
pub trait RowContext {
    fn get_value(&self, column: &str) -> Option<Value>;
}

/// Evaluate an AST expression against a row context.
pub fn eval_expr<R: RowContext>(expr: &Expr, ctx: &R) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Column(col) => Ok(ctx.get_value(col).unwrap_or(Value::Null)),
        Expr::Not(inner) => {
            let v = eval_expr(inner, ctx)?;
            Ok(Value::Bool(!v.is_truthy()))
        }
        Expr::IsNull(inner) => {
            let v = eval_expr(inner, ctx)?;
            Ok(Value::Bool(matches!(v, Value::Null)))
        }
        Expr::IsNotNull(inner) => {
            let v = eval_expr(inner, ctx)?;
            Ok(Value::Bool(!matches!(v, Value::Null)))
        }
        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let val = eval_expr(expr, ctx)?;
            let mut found = false;
            for item in list {
                let target = eval_expr(item, ctx)?;
                if values_equal(&val, &target) {
                    found = true;
                    break;
                }
            }
            Ok(Value::Bool(if *negated { !found } else { found }))
        }
        Expr::Binary { op, left, right } => {
            // Short-circuit logical ops
            if *op == BinaryOp::And {
                let l = eval_expr(left, ctx)?;
                if !l.is_truthy() {
                    return Ok(Value::Bool(false));
                }
                let r = eval_expr(right, ctx)?;
                return Ok(Value::Bool(r.is_truthy()));
            }
            if *op == BinaryOp::Or {
                let l = eval_expr(left, ctx)?;
                if l.is_truthy() {
                    return Ok(Value::Bool(true));
                }
                let r = eval_expr(right, ctx)?;
                return Ok(Value::Bool(r.is_truthy()));
            }

            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;

            match op {
                BinaryOp::Eq => Ok(Value::Bool(values_equal(&l, &r))),
                BinaryOp::NotEq => Ok(Value::Bool(!values_equal(&l, &r))),
                BinaryOp::Lt => Ok(Value::Bool(values_cmp(&l, &r).is_some_and(|c| c.is_lt()))),
                BinaryOp::LtEq => Ok(Value::Bool(values_cmp(&l, &r).is_some_and(|c| c.is_le()))),
                BinaryOp::Gt => Ok(Value::Bool(values_cmp(&l, &r).is_some_and(|c| c.is_gt()))),
                BinaryOp::GtEq => Ok(Value::Bool(values_cmp(&l, &r).is_some_and(|c| c.is_ge()))),
                BinaryOp::Like | BinaryOp::ILike => {
                    if matches!(l, Value::Null) || matches!(r, Value::Null) {
                        return Ok(Value::Bool(false));
                    }
                    let pattern = r.to_string();
                    let text = l.to_string();
                    Ok(Value::Bool(sql_like(
                        &pattern,
                        &text,
                        *op == BinaryOp::ILike,
                    )))
                }
                BinaryOp::And | BinaryOp::Or => unreachable!(),
            }
        }
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => {
            (*x as f64 - y).abs() < f64::EPSILON
        }
        (Value::Text(x), Value::Text(y)) => x == y,
        _ => false,
    }
}

fn values_cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)),
        (Value::Text(x), Value::Text(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    struct TestRow(HashMap<String, Value>);
    impl RowContext for TestRow {
        fn get_value(&self, column: &str) -> Option<Value> {
            self.0.get(column).cloned()
        }
    }

    #[test]
    fn test_sql_filter_parse_and_eval() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::Text("Alice".to_string()));
        map.insert("age".to_string(), Value::Int(30));
        map.insert("active".to_string(), Value::Bool(true));
        map.insert("A".to_string(), Value::Text("Alice".to_string()));
        let row = TestRow(map);

        let query1 = SqlFilter::parse("WHERE age >= 30 AND name LIKE 'Ali%'").unwrap();
        assert_eq!(eval_expr(&query1, &row).unwrap(), Value::Bool(true));

        let query2 = SqlFilter::parse("age < 20 OR active = true").unwrap();
        assert_eq!(eval_expr(&query2, &row).unwrap(), Value::Bool(true));

        let query3 = SqlFilter::parse("name IN ('Bob', 'Charlie')").unwrap();
        assert_eq!(eval_expr(&query3, &row).unwrap(), Value::Bool(false));

        let query4 = SqlFilter::parse("A = 'Alice'").unwrap();
        assert_eq!(eval_expr(&query4, &row).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_sql_like_patterns() {
        assert!(sql_like("%.txt", "file.txt", false));
        assert!(sql_like("test_%", "test_123", false));
        assert!(!sql_like("test_%", "other", false));
        assert!(sql_like("ALICE", "alice", true));
        assert!(!sql_like("ALICE", "alice", false));
    }

    proptest! {
        #[test]
        fn test_sql_filter_no_panic_on_arbitrary_strings(s in "\\PC*") {
            let _ = SqlFilter::parse(&s);
        }

        #[test]
        fn test_sql_like_no_panic_on_arbitrary_strings(p in "\\PC*", t in "\\PC*") {
            let _ = sql_like(&p, &t, false);
            let _ = sql_like(&p, &t, true);
        }
    }
}
