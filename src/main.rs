use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Ident(String),
    Number(String),
    String(String),
    Symbol(char),
    Operator(String),
    Eof,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    line: usize,
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut i, mut line) = (0, 1);
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            if c == '\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return Err(format!("第 {line} 行：多行注释未闭合"));
            }
            continue;
        }
        let token_line = line;
        if c == 'f' && chars.get(i + 1) == Some(&'"') {
            i += 1;
            let value = read_quoted_string(&chars, &mut i, &mut line, '"', token_line)?;
            tokens.push(Token {
                kind: TokenKind::Ident("f".into()),
                line: token_line,
            });
            tokens.push(Token {
                kind: TokenKind::String(value),
                line: token_line,
            });
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            let value = read_quoted_string(&chars, &mut i, &mut line, quote, token_line)?;
            tokens.push(Token {
                kind: TokenKind::String(value),
                line: token_line,
            });
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(|ch| ch.is_ascii_digit())
            {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number(chars[start..i].iter().collect()),
                line: token_line,
            });
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Ident(chars[start..i].iter().collect()),
                line: token_line,
            });
            continue;
        }
        if c == '<' && chars.get(i + 1) == Some(&'.') && chars.get(i + 2) == Some(&'.') {
            tokens.push(Token {
                kind: TokenKind::Operator("<..".into()),
                line: token_line,
            });
            i += 3;
            continue;
        }
        if c == '.' && chars.get(i + 1) == Some(&'.') && chars.get(i + 2) == Some(&'<') {
            tokens.push(Token {
                kind: TokenKind::Operator("..<".into()),
                line: token_line,
            });
            i += 3;
            continue;
        }
        if c == '.' && chars.get(i + 1) == Some(&'.') && chars.get(i + 2) == Some(&'.') {
            tokens.push(Token {
                kind: TokenKind::Operator("...".into()),
                line: token_line,
            });
            i += 3;
            continue;
        }
        if "+-*/%=!<>".contains(c) {
            let mut op = c.to_string();
            if (matches!(c, '+' | '-') && chars.get(i + 1) == Some(&c))
                || chars.get(i + 1) == Some(&'=')
            {
                op.push(chars[i + 1]);
                i += 1;
            }
            i += 1;
            tokens.push(Token {
                kind: TokenKind::Operator(op),
                line: token_line,
            });
            continue;
        }
        if "(){}[],;:.".contains(c) {
            tokens.push(Token {
                kind: TokenKind::Symbol(c),
                line: token_line,
            });
            i += 1;
            continue;
        }
        return Err(format!("第 {line} 行：无法识别字符 `{c}`"));
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
    });
    Ok(tokens)
}

fn read_quoted_string(
    chars: &[char],
    i: &mut usize,
    line: &mut usize,
    quote: char,
    token_line: usize,
) -> Result<String, String> {
    *i += 1;
    let mut value = String::new();
    while *i < chars.len() {
        let ch = chars[*i];
        if ch == quote {
            *i += 1;
            return Ok(value);
        }
        if ch == '\n' {
            *line += 1;
        }
        if ch == '\\' {
            *i += 1;
            value.push(decode_escape(chars, i, token_line)?);
        } else {
            value.push(ch);
            *i += 1;
        }
    }
    Err(format!("第 {token_line} 行：字符串未闭合"))
}

fn decode_escape(chars: &[char], i: &mut usize, token_line: usize) -> Result<char, String> {
    let Some(&escape) = chars.get(*i) else {
        return Err(format!("第 {token_line} 行：转义序列未完成"));
    };
    let simple = match escape {
        'n' => Some('\n'),
        't' => Some('\t'),
        '\\' => Some('\\'),
        '\'' => Some('\''),
        '"' => Some('"'),
        'r' => Some('\r'),
        'b' => Some('\u{0008}'),
        'f' => Some('\u{000c}'),
        'v' => Some('\u{000b}'),
        'a' => Some('\u{0007}'),
        '0' => None,
        '1'..='7' => None,
        'x' | 'u' | 'U' => None,
        other => {
            return Err(format!("第 {token_line} 行：未知转义序列 `\\{other}`"));
        }
    };
    if let Some(value) = simple {
        *i += 1;
        return Ok(value);
    }
    if matches!(escape, '0'..='7') {
        let start = *i;
        let mut digits = String::new();
        while *i < chars.len() && digits.len() < 3 && chars[*i].is_ascii_digit() && chars[*i] <= '7'
        {
            digits.push(chars[*i]);
            *i += 1;
        }
        let value = u32::from_str_radix(&digits, 8)
            .map_err(|_| format!("第 {token_line} 行：无效八进制转义"))?;
        return char::from_u32(value).ok_or_else(|| {
            format!(
                "第 {token_line} 行：八进制转义 `\\{}` 不是有效 Unicode 字符",
                chars[start..*i].iter().collect::<String>()
            )
        });
    }
    let (count, radix, label) = match escape {
        'x' => (2, 16, "十六进制"),
        'u' => (4, 16, "Unicode"),
        'U' => (8, 16, "Unicode"),
        _ => unreachable!(),
    };
    *i += 1;
    if *i + count > chars.len() {
        return Err(format!("第 {token_line} 行：{label}转义位数不足"));
    }
    let digits: String = chars[*i..*i + count].iter().collect();
    if !digits.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "第 {token_line} 行：无效{label}转义 `\\{escape}{digits}`"
        ));
    }
    *i += count;
    let value = u32::from_str_radix(&digits, radix)
        .map_err(|_| format!("第 {token_line} 行：无效{label}转义"))?;
    char::from_u32(value).ok_or_else(|| {
        format!("第 {token_line} 行：转义 `\\{escape}{digits}` 不是有效 Unicode 字符")
    })
}

#[derive(Clone, Debug)]
enum Expr {
    Value(Value),
    Variable(String),
    Unary(String, Box<Expr>),
    Binary(Box<Expr>, String, Box<Expr>),
    Prefix(String, Box<Expr>),
    Postfix(String, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Call(String, Vec<Expr>),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    StructInit(String, Vec<Expr>),
    Interpolated(Vec<FormatPart>),
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
    },
}

#[derive(Clone, Debug)]
enum FormatPart {
    Text(String),
    Expr(Expr),
}

#[derive(Clone, Debug)]
enum CasePattern {
    Values(Vec<Expr>),
    Range {
        lower: Expr,
        lower_inclusive: bool,
        upper: Expr,
        upper_inclusive: bool,
    },
}

#[derive(Clone, Debug)]
struct SwitchCase {
    pattern: CasePattern,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
enum Stmt {
    Var {
        name: String,
        mutable: bool,
        annotation: Option<String>,
        value: Option<Expr>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Global(Vec<String>),
    Print(Expr, bool),
    Expr(Expr),
    Return(Option<Expr>),
    Function {
        name: String,
        params: Vec<(String, Option<String>)>,
        body: Vec<Stmt>,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
        instances: Vec<StructInstance>,
    },
    Block(Vec<Stmt>),
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
    },
    Switch {
        value: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
    },
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        update: Option<Expr>,
        body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    DoWhile {
        body: Vec<Stmt>,
        condition: Expr,
    },
    Break,
}

#[derive(Clone, Debug)]
struct StructField {
    name: String,
    mutable: bool,
    annotation: Option<String>,
}

#[derive(Clone, Debug)]
struct StructDef {
    fields: Vec<StructField>,
}

#[derive(Clone, Debug)]
struct StructInstance {
    name: String,
    initializers: Vec<Vec<Expr>>,
}

#[derive(Clone, Debug)]
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Dict(Vec<(Value, Value)>),
    Struct {
        name: String,
        fields: Vec<StructMember>,
    },
}

#[derive(Clone, Debug)]
struct StructMember {
    name: String,
    value: Value,
    mutable: bool,
    annotation: Option<String>,
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

type Branches = Vec<(Expr, Vec<Stmt>)>;

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, at: 0 }
    }
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn advance(&mut self) -> Token {
        let t = self.tokens[self.at].clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.at += 1;
        }
        t
    }
    fn is_symbol(&self, c: char) -> bool {
        self.current().kind == TokenKind::Symbol(c)
    }
    fn eat_symbol(&mut self, c: char) -> bool {
        if self.is_symbol(c) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn is_ident(&self, name: &str) -> bool {
        self.current().kind == TokenKind::Ident(name.to_owned())
    }
    fn eat_ident(&mut self, name: &str) -> bool {
        if self.is_ident(name) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn error(&self, message: &str) -> String {
        format!("第 {} 行：{}", self.current().line, message)
    }
    fn expect_symbol(&mut self, c: char) -> Result<(), String> {
        if self.eat_symbol(c) {
            Ok(())
        } else {
            Err(self.error(&format!("期望 `{c}`")))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.advance().kind {
            TokenKind::Ident(s) => Ok(s),
            _ => Err(self.error("期望标识符")),
        }
    }
    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Eof) {
            body.push(self.statement()?);
        }
        Ok(body)
    }
    fn statement(&mut self) -> Result<Stmt, String> {
        if self.eat_ident("struct") {
            return self.struct_statement();
        }
        if self.eat_ident("if") {
            let condition = self.condition()?;
            let (branches, otherwise) = self.if_branches(condition)?;
            return Ok(Stmt::If {
                branches,
                otherwise,
            });
        }
        if self.eat_ident("switch") {
            return self.switch_statement();
        }
        if self.eat_ident("for") {
            return self.for_statement();
        }
        if self.eat_ident("while") {
            let condition = self.condition()?;
            return Ok(Stmt::While {
                condition,
                body: self.block()?,
            });
        }
        if self.eat_ident("do") {
            let body = self.block()?;
            if !self.eat_ident("while") {
                return Err(self.error("do 循环后期望 `while`"));
            }
            let condition = self.condition()?;
            self.eat_symbol(';');
            return Ok(Stmt::DoWhile { body, condition });
        }
        if self.eat_ident("break") {
            self.eat_symbol(';');
            return Ok(Stmt::Break);
        }
        if self.eat_ident("let") || self.eat_ident("var") {
            let mutable =
                matches!(&self.tokens[self.at - 1].kind, TokenKind::Ident(s) if s == "var");
            return self.var_statement(mutable, true);
        }
        if self.eat_ident("global") {
            let mut names = vec![self.ident()?];
            while self.eat_symbol(',') {
                names.push(self.ident()?);
            }
            self.eat_symbol(';');
            return Ok(Stmt::Global(names));
        }
        if self.eat_ident("fn") {
            let first = self.ident()?;
            let name = if self.is_symbol('(') {
                first
            } else {
                self.ident()?
            };
            self.expect_symbol('(')?;
            let mut params = Vec::new();
            if !self.is_symbol(')') {
                loop {
                    let a = self.ident()?;
                    let (annotation, param) = if self.current_is_ident() {
                        (Some(a), self.ident()?)
                    } else {
                        (None, a)
                    };
                    params.push((param, annotation));
                    if !self.eat_symbol(',') {
                        break;
                    }
                }
            }
            self.expect_symbol(')')?;
            let body = self.block()?;
            return Ok(Stmt::Function { name, params, body });
        }
        if self.eat_ident("return") {
            let value = if self.is_symbol('(') {
                self.advance();
                let expr = if self.is_symbol(')') {
                    None
                } else {
                    Some(self.expression(0)?)
                };
                self.expect_symbol(')')?;
                expr
            } else if self.is_symbol(';') || self.is_symbol('}') {
                None
            } else {
                Some(self.expression(0)?)
            };
            self.eat_symbol(';');
            return Ok(Stmt::Return(value));
        }
        if self.eat_ident("println") || self.eat_ident("print") {
            let newline =
                matches!(&self.tokens[self.at - 1].kind, TokenKind::Ident(s) if s == "println");
            self.expect_symbol('(')?;
            let value = self.expression(0)?;
            self.expect_symbol(')')?;
            self.eat_symbol(';');
            return Ok(Stmt::Print(value, newline));
        }
        if self.is_symbol('{') {
            return Ok(Stmt::Block(self.block()?));
        }
        if let Some(stmt) = self.assignment_statement(false)? {
            self.eat_symbol(';');
            return Ok(stmt);
        }
        let expr = self.expression(0)?;
        self.eat_symbol(';');
        Ok(Stmt::Expr(expr))
    }
    fn var_statement(&mut self, mutable: bool, consume_semicolon: bool) -> Result<Stmt, String> {
        let first = self.ident()?;
        let (annotation, name) = if self.current_is_ident() {
            (Some(first), self.ident()?)
        } else {
            (None, first)
        };
        let value = if self.is_operator("=") {
            self.advance();
            Some(self.expression(0)?)
        } else if annotation.is_some() && self.is_symbol('{') {
            Some(self.struct_initializer(annotation.as_ref().unwrap())?)
        } else {
            None
        };
        if value.is_none() && !self.is_symbol(';') {
            return Err(self.error("变量声明需要 `=` 或 `;`"));
        }
        if consume_semicolon {
            self.eat_symbol(';');
        }
        Ok(Stmt::Var {
            name,
            mutable,
            annotation,
            value,
        })
    }
    fn struct_statement(&mut self) -> Result<Stmt, String> {
        let name = self.ident()?;
        self.expect_symbol('{')?;
        let mut fields = Vec::new();
        while !self.is_symbol('}') && !matches!(self.current().kind, TokenKind::Eof) {
            let mutable = if self.eat_ident("let") {
                false
            } else if self.eat_ident("var") {
                true
            } else {
                return Err(self.error("结构体成员需要 `let` 或 `var`"));
            };
            let first = self.ident()?;
            let (annotation, field_name) = if self.current_is_ident() {
                (Some(first), self.ident()?)
            } else {
                (None, first)
            };
            self.expect_symbol(';')?;
            fields.push(StructField {
                name: field_name,
                mutable,
                annotation,
            });
        }
        self.expect_symbol('}')?;
        let instances = self.struct_instances()?;
        if instances.is_empty() {
            self.eat_symbol(';');
        } else if !self.eat_symbol(';') {
            return Err(self.error("结构体实例声明后需要 `;`"));
        }
        Ok(Stmt::Struct {
            name,
            fields,
            instances,
        })
    }
    fn struct_instances(&mut self) -> Result<Vec<StructInstance>, String> {
        let mut instances = Vec::new();
        if self.is_symbol(';') || !self.current_is_ident() {
            return Ok(instances);
        }
        loop {
            let mut name = self.ident()?;
            let mut initializers = Vec::new();
            while self.is_symbol('{') || self.is_symbol('[') {
                let mut values = Vec::new();
                let closing = if self.eat_symbol('{') {
                    '}'
                } else {
                    self.advance();
                    ']'
                };
                if !self.is_symbol(closing) {
                    loop {
                        values.push(self.expression(0)?);
                        if !self.eat_symbol(',') {
                            break;
                        }
                    }
                }
                self.expect_symbol(closing)?;
                initializers.push(values);
                if self.current_is_ident() {
                    name = self.ident()?;
                }
            }
            instances.push(StructInstance { name, initializers });
            if !self.eat_symbol(',') {
                break;
            }
            if !self.current_is_ident() {
                return Err(self.error("逗号后期望结构体实例名"));
            }
        }
        Ok(instances)
    }
    fn struct_initializer(&mut self, name: &str) -> Result<Expr, String> {
        self.expect_symbol('{')?;
        let mut values = Vec::new();
        if !self.is_symbol('}') {
            loop {
                values.push(self.expression(0)?);
                if !self.eat_symbol(',') {
                    break;
                }
            }
        }
        self.expect_symbol('}')?;
        Ok(Expr::StructInit(name.to_owned(), values))
    }
    fn assignment_statement(&mut self, consume_semicolon: bool) -> Result<Option<Stmt>, String> {
        let start = self.at;
        let target = match self.expression(0) {
            Ok(target) => target,
            Err(_) => {
                self.at = start;
                return Ok(None);
            }
        };
        let Some(Token {
            kind: TokenKind::Operator(op),
            ..
        }) = self.tokens.get(self.at)
        else {
            self.at = start;
            return Ok(None);
        };
        if !matches!(op.as_str(), "=" | "+=" | "-=" | "*=" | "/=" | "%=") {
            self.at = start;
            return Ok(None);
        }
        let op = match self.advance().kind {
            TokenKind::Operator(op) => op,
            _ => unreachable!(),
        };
        let value = self.expression(0)?;
        if consume_semicolon {
            self.eat_symbol(';');
        }
        let value = if op == "=" {
            value
        } else {
            Expr::Binary(
                Box::new(target.clone()),
                op[..1].to_owned(),
                Box::new(value),
            )
        };
        Ok(Some(Stmt::Assign { target, value }))
    }
    fn for_statement(&mut self) -> Result<Stmt, String> {
        self.expect_symbol('(')?;
        let init = if self.eat_symbol(';') {
            None
        } else if self.eat_ident("let") || self.eat_ident("var") {
            let mutable =
                matches!(&self.tokens[self.at - 1].kind, TokenKind::Ident(s) if s == "var");
            let stmt = self.var_statement(mutable, false)?;
            self.expect_symbol(';')?;
            Some(Box::new(stmt))
        } else {
            let stmt = if let Some(stmt) = self.assignment_statement(false)? {
                stmt
            } else {
                Stmt::Expr(self.expression(0)?)
            };
            self.expect_symbol(';')?;
            Some(Box::new(stmt))
        };
        let condition = if self.is_symbol(';') {
            None
        } else {
            Some(self.expression(0)?)
        };
        self.expect_symbol(';')?;
        let update = if self.is_symbol(')') {
            None
        } else {
            Some(self.expression(0)?)
        };
        self.expect_symbol(')')?;
        Ok(Stmt::For {
            init,
            condition,
            update,
            body: self.block()?,
        })
    }
    fn current_is_ident(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(_))
    }
    fn is_operator(&self, op: &str) -> bool {
        self.current().kind == TokenKind::Operator(op.to_owned())
    }
    fn condition(&mut self) -> Result<Expr, String> {
        self.expect_symbol('(')?;
        let condition = self.expression(0)?;
        self.expect_symbol(')')?;
        Ok(condition)
    }
    fn if_branches(&mut self, first: Expr) -> Result<(Branches, Option<Vec<Stmt>>), String> {
        let mut branches = Vec::new();
        let mut condition = first;
        loop {
            branches.push((condition, self.block()?));
            if self.eat_ident("elif") {
                condition = self.condition()?;
            } else if self.eat_ident("else") {
                if self.eat_ident("if") {
                    condition = self.condition()?;
                } else {
                    return Ok((branches, Some(self.block()?)));
                }
            } else {
                return Ok((branches, None));
            }
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect_symbol('{')?;
        let mut body = Vec::new();
        while !self.is_symbol('}') && !matches!(self.current().kind, TokenKind::Eof) {
            body.push(self.statement()?);
        }
        self.expect_symbol('}')?;
        Ok(body)
    }
    fn switch_statement(&mut self) -> Result<Stmt, String> {
        self.expect_symbol('(')?;
        let value = self.expression(0)?;
        self.expect_symbol(')')?;
        self.expect_symbol('{')?;
        let mut cases = Vec::new();
        let mut default = None;
        while !self.is_symbol('}') && !matches!(self.current().kind, TokenKind::Eof) {
            if self.eat_ident("case") {
                if default.is_some() {
                    return Err(self.error("case 不能出现在 default 之后"));
                }
                let pattern = self.case_pattern()?;
                let body = self.block()?;
                cases.push(SwitchCase { pattern, body });
            } else if self.eat_ident("default") {
                if default.is_some() {
                    return Err(self.error("switch 中只能有一个 default"));
                }
                default = Some(self.block()?);
            } else {
                return Err(self.error("switch 中期望 case、default 或 `}`"));
            }
        }
        self.expect_symbol('}')?;
        Ok(Stmt::Switch {
            value,
            cases,
            default,
        })
    }
    fn case_pattern(&mut self) -> Result<CasePattern, String> {
        self.expect_symbol('(')?;
        let first = self.expression(0)?;
        if let TokenKind::Operator(op) = &self.current().kind {
            if ["...", "<..", "..<"].contains(&op.as_str()) {
                let op = op.clone();
                self.advance();
                let upper = self.expression(0)?;
                self.expect_symbol(')')?;
                let (lower_inclusive, upper_inclusive) = match op.as_str() {
                    "..." => (true, true),
                    "<.." => (false, true),
                    _ => (true, false),
                };
                return Ok(CasePattern::Range {
                    lower: first,
                    lower_inclusive,
                    upper,
                    upper_inclusive,
                });
            }
        }
        let mut values = vec![first];
        while self.eat_symbol(',') {
            values.push(self.expression(0)?);
        }
        self.expect_symbol(')')?;
        Ok(CasePattern::Values(values))
    }
    fn expression(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = if self.is_operator("-") || self.is_operator("!") {
            let op = match self.advance().kind {
                TokenKind::Operator(s) => s,
                _ => unreachable!(),
            };
            Expr::Unary(op, Box::new(self.expression(7)?))
        } else if self.is_operator("++") || self.is_operator("--") {
            let op = match self.advance().kind {
                TokenKind::Operator(s) => s,
                _ => unreachable!(),
            };
            let operand = self.expression(7)?;
            Expr::Prefix(op, Box::new(operand))
        } else {
            self.primary()?
        };
        loop {
            if matches!(self.current().kind, TokenKind::Operator(ref op) if op == "++" || op == "--")
            {
                let op = match self.advance().kind {
                    TokenKind::Operator(s) => s,
                    _ => unreachable!(),
                };
                left = Expr::Postfix(op, Box::new(left));
                continue;
            }
            let op = match &self.current().kind {
                TokenKind::Operator(op) => op.clone(),
                _ => break,
            };
            let prec = match op.as_str() {
                "==" | "!=" | ">" | ">=" | "<" | "<=" => 1,
                "+" | "-" => 2,
                "*" | "/" | "%" => 3,
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.expression(prec + 1)?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }
    fn primary(&mut self) -> Result<Expr, String> {
        let token = self.advance();
        let expr = match token.kind {
            TokenKind::Number(n) => {
                if n.contains('.') {
                    Ok(Expr::Value(Value::Float(
                        n.parse()
                            .map_err(|_| format!("第 {} 行：无效数字", token.line))?,
                    )))
                } else {
                    Ok(Expr::Value(Value::Int(n.parse().map_err(|_| {
                        format!("第 {} 行：整数超出范围", token.line)
                    })?)))
                }
            }
            TokenKind::String(s) => Ok(Expr::Value(Value::String(s))),
            TokenKind::Ident(name) if name == "true" => Ok(Expr::Value(Value::Bool(true))),
            TokenKind::Ident(name) if name == "false" => Ok(Expr::Value(Value::Bool(false))),
            TokenKind::Ident(name) if name == "null" => Ok(Expr::Value(Value::Null)),
            TokenKind::Ident(name) if name == "if" => {
                let condition = self.condition()?;
                let (branches, otherwise) = self.if_branches(condition)?;
                Ok(Expr::If {
                    branches,
                    otherwise,
                })
            }
            TokenKind::Ident(name)
                if name == "f" && matches!(&self.current().kind, TokenKind::String(_)) =>
            {
                let raw = match self.advance().kind {
                    TokenKind::String(s) => s,
                    _ => unreachable!(),
                };
                Ok(Expr::Interpolated(parse_format(&raw)?))
            }
            TokenKind::Ident(name) => {
                if self.eat_symbol('(') {
                    let mut args = Vec::new();
                    if !self.is_symbol(')') {
                        loop {
                            args.push(self.expression(0)?);
                            if !self.eat_symbol(',') {
                                break;
                            }
                        }
                    }
                    self.expect_symbol(')')?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            TokenKind::Symbol('(') => {
                let e = self.expression(0)?;
                self.expect_symbol(')')?;
                Ok(e)
            }
            TokenKind::Symbol('[') => {
                let mut values = Vec::new();
                if !self.is_symbol(']') {
                    loop {
                        values.push(self.expression(0)?);
                        if !self.eat_symbol(',') {
                            break;
                        }
                    }
                }
                self.expect_symbol(']')?;
                Ok(Expr::List(values))
            }
            TokenKind::Symbol('{') => {
                let mut entries = Vec::new();
                if !self.is_symbol('}') {
                    loop {
                        let key = self.expression(0)?;
                        self.expect_symbol(':')?;
                        let value = self.expression(0)?;
                        entries.push((key, value));
                        if !self.eat_symbol(',') {
                            break;
                        }
                    }
                }
                self.expect_symbol('}')?;
                Ok(Expr::Dict(entries))
            }
            other => Err(format!("第 {} 行：此处不能使用 {:?}", token.line, other)),
        }?;
        let mut expr = expr;
        loop {
            if self.eat_symbol('[') {
                let index = self.expression(0)?;
                self.expect_symbol(']')?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
            } else if self.eat_symbol('.') {
                let field = self.ident()?;
                expr = Expr::Member(Box::new(expr), field);
            } else {
                break;
            }
        }
        Ok(expr)
    }
}

fn parse_format(raw: &str) -> Result<Vec<FormatPart>, String> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                text.push('{');
                continue;
            }
            let mut name = String::new();
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '}' {
                    break;
                }
                name.push(ch);
            }
            if !text.is_empty() {
                parts.push(FormatPart::Text(std::mem::take(&mut text)));
            }
            if !name.is_empty() {
                let tokens = lex(name.trim())?;
                let mut parser = Parser::new(tokens);
                let expr = parser.expression(0)?;
                if !matches!(parser.current().kind, TokenKind::Eof) {
                    return Err("格式化表达式未完整解析".into());
                }
                parts.push(FormatPart::Expr(expr));
            }
        } else if c == '}' && chars.peek() == Some(&'}') {
            chars.next();
            text.push('}');
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        parts.push(FormatPart::Text(text));
    }
    Ok(parts)
}

#[derive(Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}
#[derive(Clone)]
struct Function {
    params: Vec<(String, Option<String>)>,
    body: Vec<Stmt>,
}
enum Flow {
    Continue,
    Return(Value),
    Break,
}

struct Interpreter {
    scopes: Vec<Vec<(String, Binding)>>,
    globals: Vec<(String, Binding)>,
    global_names: Vec<Vec<String>>,
    functions: Vec<(String, Function)>,
    structs: Vec<(String, StructDef)>,
    call_depth: usize,
}

impl Interpreter {
    fn new() -> Self {
        Self {
            scopes: vec![Vec::new()],
            globals: Vec::new(),
            global_names: vec![Vec::new()],
            functions: Vec::new(),
            structs: Vec::new(),
            call_depth: 0,
        }
    }
    fn run(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for stmt in statements {
            match self.execute(stmt)? {
                Flow::Continue => {}
                Flow::Return(_) => return Err("return 不能出现在函数外部".into()),
                Flow::Break => return Err("break 不能出现在 switch 外部".into()),
            }
        }
        Ok(())
    }
    fn execute(&mut self, stmt: &Stmt) -> Result<Flow, String> {
        match stmt {
            Stmt::Var {
                name,
                mutable,
                annotation,
                value,
            } => {
                if value.is_none()
                    && annotation
                        .as_deref()
                        .is_some_and(|ty| self.structs.iter().any(|(n, _)| n == ty))
                {
                    return Err(format!(
                        "结构体 `{}` 必须在对象创建时初始化",
                        annotation.as_deref().unwrap()
                    ));
                }
                let value = if let Some(value) = value {
                    self.eval(value)?
                } else {
                    Value::Null
                };
                self.check_type(annotation.as_deref(), &value)?;
                let globals = self.global_names.last().cloned().unwrap_or_default();
                if globals.contains(name) {
                    self.define_global(
                        name,
                        Binding {
                            value,
                            mutable: *mutable,
                        },
                    )?;
                } else {
                    self.define_local(
                        name,
                        Binding {
                            value,
                            mutable: *mutable,
                        },
                    )?;
                }
            }
            Stmt::Assign { target, value } => {
                let new_value = self.eval(value)?;
                self.assign_target(target, new_value)?;
            }
            Stmt::Global(names) => {
                let frame = self.global_names.last_mut().unwrap();
                for name in names {
                    if !frame.contains(name) {
                        frame.push(name.clone());
                    }
                }
            }
            Stmt::Print(expr, newline) => {
                let value = self.eval(expr)?;
                if *newline {
                    println!("{}", value_string(&value));
                } else {
                    print!("{}", value_string(&value));
                    io::stdout().flush().map_err(|e| e.to_string())?;
                }
            }
            Stmt::Expr(expr) => {
                self.eval(expr)?;
            }
            Stmt::Return(expr) => {
                return Ok(Flow::Return(if let Some(expr) = expr {
                    self.eval(expr)?
                } else {
                    Value::Null
                }))
            }
            Stmt::Function { name, params, body } => {
                if self.functions.iter().any(|(n, _)| n == name) {
                    return Err(format!("函数 `{name}` 已定义"));
                }
                self.functions.push((
                    name.clone(),
                    Function {
                        params: params.clone(),
                        body: body.clone(),
                    },
                ));
            }
            Stmt::Struct {
                name,
                fields,
                instances,
            } => {
                if self.structs.iter().any(|(n, _)| n == name) {
                    return Err(format!("结构体 `{name}` 已定义"));
                }
                self.structs.push((
                    name.clone(),
                    StructDef {
                        fields: fields.clone(),
                    },
                ));
                for instance in instances {
                    let value = self.build_struct_value(name, fields, &instance.initializers)?;
                    self.define_global(
                        &instance.name,
                        Binding {
                            value,
                            mutable: true,
                        },
                    )?;
                }
            }
            Stmt::Block(body) => {
                self.scopes.push(Vec::new());
                self.global_names.push(Vec::new());
                let result = self.execute_block(body);
                self.scopes.pop();
                self.global_names.pop();
                return result;
            }
            Stmt::If {
                branches,
                otherwise,
            } => {
                if let Some(body) = self.select_branch(branches, otherwise.as_ref())? {
                    self.scopes.push(Vec::new());
                    self.global_names.push(Vec::new());
                    let result = self.execute_block(body);
                    self.scopes.pop();
                    self.global_names.pop();
                    return result;
                }
            }
            Stmt::Switch {
                value,
                cases,
                default,
            } => {
                let value = self.eval(value)?;
                return self.execute_switch(&value, cases, default.as_ref());
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
            } => {
                self.scopes.push(Vec::new());
                self.global_names.push(Vec::new());
                if let Some(init) = init {
                    match self.execute(init)? {
                        Flow::Continue => {}
                        Flow::Return(value) => {
                            self.scopes.pop();
                            self.global_names.pop();
                            return Ok(Flow::Return(value));
                        }
                        Flow::Break => {
                            self.scopes.pop();
                            self.global_names.pop();
                            return Ok(Flow::Continue);
                        }
                    }
                }
                loop {
                    if let Some(condition) = condition {
                        let value = self.eval(condition)?;
                        if !matches!(value, Value::Bool(_)) {
                            return Err("for 条件必须是 bool 值".into());
                        }
                        if !matches!(value, Value::Bool(true)) {
                            break;
                        }
                    }
                    match self.execute_loop_body(body)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(value) => {
                            self.scopes.pop();
                            self.global_names.pop();
                            return Ok(Flow::Return(value));
                        }
                    }
                    if let Some(update) = update {
                        self.eval(update)?;
                    }
                }
                self.scopes.pop();
                self.global_names.pop();
            }
            Stmt::While { condition, body } => loop {
                let value = self.eval(condition)?;
                if !matches!(value, Value::Bool(_)) {
                    return Err("while 条件必须是 bool 值".into());
                }
                if !matches!(value, Value::Bool(true)) {
                    break;
                }
                match self.execute_loop_body(body)? {
                    Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                }
            },
            Stmt::DoWhile { body, condition } => loop {
                match self.execute_loop_body(body)? {
                    Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                }
                let value = self.eval(condition)?;
                if !matches!(value, Value::Bool(_)) {
                    return Err("do while 条件必须是 bool 值".into());
                }
                if !matches!(value, Value::Bool(true)) {
                    break;
                }
            },
            Stmt::Break => return Ok(Flow::Break),
        }
        Ok(Flow::Continue)
    }
    fn execute_block(&mut self, body: &[Stmt]) -> Result<Flow, String> {
        for stmt in body {
            match self.execute(stmt)? {
                Flow::Continue => {}
                ret @ Flow::Return(_) => return Ok(ret),
                ret @ Flow::Break => return Ok(ret),
            }
        }
        Ok(Flow::Continue)
    }
    fn execute_loop_body(&mut self, body: &[Stmt]) -> Result<Flow, String> {
        self.scopes.push(Vec::new());
        self.global_names.push(Vec::new());
        let result = self.execute_block(body);
        self.scopes.pop();
        self.global_names.pop();
        result
    }
    fn execute_switch(
        &mut self,
        value: &Value,
        cases: &[SwitchCase],
        default: Option<&Vec<Stmt>>,
    ) -> Result<Flow, String> {
        let mut first_match = None;
        for (index, case) in cases.iter().enumerate() {
            if self.case_matches(value, &case.pattern)? {
                first_match = Some(index);
                break;
            }
        }
        if let Some(index) = first_match {
            for case in &cases[index..] {
                match self.execute_case_body(&case.body)? {
                    Flow::Continue => {}
                    Flow::Break => return Ok(Flow::Continue),
                    ret @ Flow::Return(_) => return Ok(ret),
                }
            }
        }
        if let Some(body) = default {
            match self.execute_case_body(body)? {
                Flow::Break | Flow::Continue => {}
                ret @ Flow::Return(_) => return Ok(ret),
            }
        }
        Ok(Flow::Continue)
    }
    fn execute_case_body(&mut self, body: &[Stmt]) -> Result<Flow, String> {
        self.scopes.push(Vec::new());
        self.global_names.push(Vec::new());
        let result = self.execute_block(body);
        self.scopes.pop();
        self.global_names.pop();
        result
    }
    fn case_matches(&mut self, value: &Value, pattern: &CasePattern) -> Result<bool, String> {
        match pattern {
            CasePattern::Values(values) => {
                for candidate in values {
                    let candidate = self.eval(candidate)?;
                    if matches!(binary(value.clone(), "==", candidate)?, Value::Bool(true)) {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            CasePattern::Range {
                lower,
                lower_inclusive,
                upper,
                upper_inclusive,
            } => {
                let lower = self.eval(lower)?;
                let upper = self.eval(upper)?;
                let lower_op = if *lower_inclusive { ">=" } else { ">" };
                let upper_op = if *upper_inclusive { "<=" } else { "<" };
                let lower_ok = matches!(binary(value.clone(), lower_op, lower)?, Value::Bool(true));
                let upper_ok = matches!(binary(value.clone(), upper_op, upper)?, Value::Bool(true));
                Ok(lower_ok && upper_ok)
            }
        }
    }
    fn select_branch<'a>(
        &mut self,
        branches: &'a [(Expr, Vec<Stmt>)],
        otherwise: Option<&'a Vec<Stmt>>,
    ) -> Result<Option<&'a Vec<Stmt>>, String> {
        for (condition, body) in branches {
            let value = self.eval(condition)?;
            if !matches!(value, Value::Bool(_)) {
                return Err("if 条件必须是 bool 值".into());
            }
            if matches!(value, Value::Bool(true)) {
                return Ok(Some(body));
            }
        }
        Ok(otherwise)
    }
    fn eval_branch(&mut self, body: &[Stmt]) -> Result<Value, String> {
        self.scopes.push(Vec::new());
        self.global_names.push(Vec::new());
        let mut result = Value::Null;
        for stmt in body {
            match stmt {
                Stmt::Expr(expr) => result = self.eval(expr)?,
                _ => match self.execute(stmt)? {
                    Flow::Continue => {}
                    Flow::Return(_) => {
                        self.scopes.pop();
                        self.global_names.pop();
                        return Err("if 表达式分支不能包含 return".into());
                    }
                    Flow::Break => {
                        self.scopes.pop();
                        self.global_names.pop();
                        return Err("if 表达式分支不能包含 break".into());
                    }
                },
            }
        }
        self.scopes.pop();
        self.global_names.pop();
        Ok(result)
    }
    fn eval(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Value(v) => Ok(v.clone()),
            Expr::Variable(name) => self
                .lookup(name)
                .cloned()
                .ok_or_else(|| format!("变量 `{name}` 未定义")),
            Expr::Unary(op, expr) => {
                let v = self.eval(expr)?;
                match (op.as_str(), v) {
                    ("-", Value::Int(n)) => n
                        .checked_neg()
                        .map(Value::Int)
                        .ok_or_else(|| "整数溢出".to_owned()),
                    ("-", Value::Float(n)) => Ok(Value::Float(-n)),
                    ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
                    _ => Err(format!("运算符 `{op}` 的操作数类型不正确")),
                }
            }
            Expr::Prefix(op, expr) => self.eval_update(expr, op, true),
            Expr::Postfix(op, expr) => self.eval_update(expr, op, false),
            Expr::Binary(left, op, right) => {
                let a = self.eval(left)?;
                let b = self.eval(right)?;
                binary(a, op, b)
            }
            Expr::Index(container, index) => {
                let container = self.eval(container)?;
                let index = self.eval(index)?;
                index_value(&container, &index)
            }
            Expr::Member(base, field) => {
                let value = self.eval(base)?;
                member_value(&value, field)
            }
            Expr::List(values) => {
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    result.push(self.eval(value)?);
                }
                Ok(Value::List(result))
            }
            Expr::Dict(entries) => {
                let mut result = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    result.push((self.eval(key)?, self.eval(value)?));
                }
                Ok(Value::Dict(result))
            }
            Expr::StructInit(name, values) => {
                let def = self
                    .structs
                    .iter()
                    .find(|(struct_name, _)| struct_name == name)
                    .map(|(_, def)| def.clone())
                    .ok_or_else(|| format!("结构体 `{name}` 未定义"))?;
                let initializers = if values.is_empty() {
                    Vec::new()
                } else {
                    vec![values.clone()]
                };
                self.build_struct_value(name, &def.fields, &initializers)
            }
            Expr::Interpolated(parts) => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        FormatPart::Text(t) => s.push_str(t),
                        FormatPart::Expr(e) => s.push_str(&value_string(&self.eval(e)?)),
                    }
                }
                Ok(Value::String(s))
            }
            Expr::If {
                branches,
                otherwise,
            } => {
                if let Some(body) = self.select_branch(branches, otherwise.as_ref())? {
                    self.eval_branch(body)
                } else {
                    Ok(Value::Null)
                }
            }
            Expr::Call(name, args) => {
                if matches!(
                    name.as_str(),
                    "int" | "float" | "string" | "bool" | "type" | "size"
                ) {
                    return self.eval_builtin(name, args);
                }
                if name == "readln" {
                    if args.len() > 1 {
                        return Err("readln 最多接受一个提示字符串".into());
                    }
                    if let Some(prompt) = args.first() {
                        print!("{}", value_string(&self.eval(prompt)?));
                        io::stdout().flush().map_err(|e| e.to_string())?;
                    }
                    let mut line = String::new();
                    io::stdin()
                        .read_line(&mut line)
                        .map_err(|e| e.to_string())?;
                    return Ok(Value::String(
                        line.trim_end_matches(['\r', '\n']).to_owned(),
                    ));
                }
                let function = self
                    .functions
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, f)| f.clone())
                    .ok_or_else(|| format!("函数 `{name}` 未定义"))?;
                if args.len() != function.params.len() {
                    return Err(format!(
                        "函数 `{name}` 需要 {} 个参数，收到 {} 个",
                        function.params.len(),
                        args.len()
                    ));
                }
                if self.call_depth >= 256 {
                    return Err("函数调用超过最大深度 (256)".into());
                }
                let mut values = Vec::new();
                for (arg, (param, annotation)) in args.iter().zip(function.params.iter()) {
                    let v = self.eval(arg)?;
                    self.check_type(annotation.as_deref(), &v)
                        .map_err(|e| format!("参数 `{param}`：{e}"))?;
                    values.push(v);
                }
                self.call_depth += 1;
                self.scopes.push(Vec::new());
                self.global_names.push(Vec::new());
                for ((param, _), value) in function.params.iter().zip(values) {
                    self.define_local(
                        param,
                        Binding {
                            value,
                            mutable: true,
                        },
                    )?;
                }
                let result = self.execute_block(&function.body);
                self.scopes.pop();
                self.global_names.pop();
                self.call_depth -= 1;
                match result? {
                    Flow::Continue => Ok(Value::Null),
                    Flow::Return(v) => Ok(v),
                    Flow::Break => Err("break 不在 switch 中".into()),
                }
            }
        }
    }
    fn build_struct_value(
        &mut self,
        name: &str,
        fields: &[StructField],
        initializers: &[Vec<Expr>],
    ) -> Result<Value, String> {
        let expressions: Option<&Vec<Expr>> = match initializers {
            [] => None,
            [values] => {
                if values.len() != fields.len() {
                    return Err(format!(
                        "结构体 `{name}` 需要 {} 个成员值，收到 {} 个",
                        fields.len(),
                        values.len()
                    ));
                }
                Some(values)
            }
            _ => {
                return Err(format!(
                    "结构体 `{name}` 需要 {} 个成员值，收到 {} 组",
                    fields.len(),
                    initializers.len()
                ));
            }
        };
        let mut members = Vec::with_capacity(fields.len());
        for (index, field) in fields.iter().enumerate() {
            let value = match expressions {
                Some(values) => self.eval(&values[index])?,
                None => default_member_value(field.annotation.as_deref()),
            };
            self.check_type(field.annotation.as_deref(), &value)
                .map_err(|error| format!("成员 `{}`：{error}", field.name))?;
            members.push(StructMember {
                name: field.name.clone(),
                value,
                mutable: field.mutable,
                annotation: field.annotation.clone(),
            });
        }
        Ok(Value::Struct {
            name: name.to_owned(),
            fields: members,
        })
    }
    fn assign_target(&mut self, target: &Expr, value: Value) -> Result<(), String> {
        match target {
            Expr::Variable(name) => {
                let Some(binding) = self.lookup_mut(name) else {
                    return Err(format!("变量 `{name}` 未定义"));
                };
                if !binding.mutable {
                    return Err(format!("固定变量 `{name}` 不能重新赋值"));
                }
                binding.value = value;
                Ok(())
            }
            Expr::Member(base, field) => self.assign_member(base, field, value),
            _ => Err("赋值目标必须是变量或结构体成员".into()),
        }
    }
    fn assign_member(&mut self, base: &Expr, field: &str, value: Value) -> Result<(), String> {
        if let Expr::Variable(name) = base {
            let (annotation, mutable, binding_mutable) = {
                let Some(binding) = self.lookup_binding(name) else {
                    return Err(format!("变量 `{name}` 未定义"));
                };
                let Value::Struct {
                    name: struct_name,
                    fields,
                } = &binding.value
                else {
                    return Err("成员访问只能用于结构体".into());
                };
                let Some(member) = fields.iter().find(|member| member.name == field) else {
                    return Err(format!("结构体 `{struct_name}` 没有成员 `{field}`"));
                };
                (member.annotation.clone(), member.mutable, binding.mutable)
            };
            if !binding_mutable {
                return Err(format!("固定变量 `{name}` 不能修改成员"));
            }
            if !mutable {
                return Err(format!("固定成员 `{field}` 不能重新赋值"));
            }
            self.check_type(annotation.as_deref(), &value)
                .map_err(|error| format!("成员 `{field}`：{error}"))?;
            let binding = self.lookup_mut(name).expect("binding disappeared");
            let Value::Struct { fields, .. } = &mut binding.value else {
                unreachable!("binding changed during member assignment");
            };
            let member = fields
                .iter_mut()
                .find(|member| member.name == field)
                .expect("member disappeared during assignment");
            member.value = value;
            return Ok(());
        }

        let mut updated = self.eval(base)?;
        let Value::Struct {
            name: struct_name,
            fields,
        } = &mut updated
        else {
            return Err("成员访问只能用于结构体".into());
        };
        let Some(member) = fields.iter_mut().find(|member| member.name == field) else {
            return Err(format!("结构体 `{struct_name}` 没有成员 `{field}`"));
        };
        if !member.mutable {
            return Err(format!("固定成员 `{field}` 不能重新赋值"));
        }
        self.check_type(member.annotation.as_deref(), &value)
            .map_err(|error| format!("成员 `{field}`：{error}"))?;
        member.value = value;
        self.assign_target(base, updated)
    }
    fn eval_update(&mut self, expr: &Expr, op: &str, prefix: bool) -> Result<Value, String> {
        let Expr::Variable(name) = expr else {
            return Err("++ 和 -- 只能用于变量".into());
        };
        let Some(binding) = self.lookup_mut(name) else {
            return Err(format!("变量 `{name}` 未定义"));
        };
        if !binding.mutable {
            return Err(format!("固定变量 `{name}` 不能重新赋值"));
        }
        let old = binding.value.clone();
        let next = match (&old, op) {
            (Value::Int(n), "++") => n
                .checked_add(1)
                .map(Value::Int)
                .ok_or_else(|| "整数溢出".to_owned())?,
            (Value::Int(n), "--") => n
                .checked_sub(1)
                .map(Value::Int)
                .ok_or_else(|| "整数溢出".to_owned())?,
            (Value::Float(n), "++") => Value::Float(n + 1.0),
            (Value::Float(n), "--") => Value::Float(n - 1.0),
            _ => return Err(format!("运算符 `{op}` 的操作数类型不正确")),
        };
        binding.value = next.clone();
        if prefix {
            Ok(next)
        } else {
            Ok(old)
        }
    }
    fn eval_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err(format!(
                "内置函数 `{name}` 需要 1 个参数，收到 {} 个",
                args.len()
            ));
        }
        let value = self.eval(&args[0])?;
        match name {
            "type" => Ok(Value::String(value_type(&value))),
            "size" => match value {
                Value::List(values) => Ok(Value::Int(values.len() as i64)),
                Value::Dict(entries) => Ok(Value::Int(entries.len() as i64)),
                _ => Err("size 只能用于列表或字典".into()),
            },
            "int" => convert_int(value),
            "float" => convert_float(value),
            "string" => convert_string(value),
            "bool" => convert_bool(value),
            _ => unreachable!(),
        }
    }
    fn define_local(&mut self, name: &str, binding: Binding) -> Result<(), String> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.iter().any(|(n, _)| n == name) {
            return Err(format!("变量 `{name}` 已在此作用域定义"));
        }
        scope.push((name.to_owned(), binding));
        Ok(())
    }
    fn define_global(&mut self, name: &str, binding: Binding) -> Result<(), String> {
        if self.globals.iter().any(|(n, _)| n == name) {
            return Err(format!("全局变量 `{name}` 已定义"));
        }
        self.globals.push((name.to_owned(), binding));
        Ok(())
    }
    fn lookup(&self, name: &str) -> Option<&Value> {
        self.lookup_binding(name).map(|binding| &binding.value)
    }
    fn lookup_binding(&self, name: &str) -> Option<&Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.iter().rev().find(|(n, _)| n == name).map(|(_, b)| b))
            .or_else(|| self.globals.iter().find(|(n, _)| n == name).map(|(_, b)| b))
    }
    fn lookup_mut(&mut self, name: &str) -> Option<&mut Binding> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((_, binding)) = scope.iter_mut().rev().find(|(n, _)| n == name) {
                return Some(binding);
            }
        }
        self.globals
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b)
    }
    fn check_type(&self, annotation: Option<&str>, value: &Value) -> Result<(), String> {
        let Some(ty) = annotation else {
            return Ok(());
        };
        if matches!(value, Value::Null) {
            return Ok(());
        }
        let valid = match ty {
            "int" => matches!(value, Value::Int(_)),
            "float" => matches!(value, Value::Float(_)),
            "string" => matches!(value, Value::String(_)),
            "bool" => matches!(value, Value::Bool(_)),
            "void" => matches!(value, Value::Null),
            ty if self.structs.iter().any(|(name, _)| name == ty) => {
                matches!(value, Value::Struct { name, .. } if name == ty)
            }
            _ => return Err(format!("未知类型 `{ty}`")),
        };
        if valid {
            Ok(())
        } else {
            Err(format!("类型 `{ty}` 与值 `{}` 不匹配", value_string(value)))
        }
    }
}

fn binary(a: Value, op: &str, b: Value) -> Result<Value, String> {
    if op == "+" {
        if let (Value::String(x), Value::String(y)) = (&a, &b) {
            return Ok(Value::String(format!("{x}{y}")));
        }
    }
    if ["==", "!=", ">", ">=", "<", "<="].contains(&op) {
        let ordering = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Some(x.partial_cmp(y).unwrap()),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
            (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
            (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
            (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
            _ => None,
        };
        if op == "==" {
            return Ok(Value::Bool(ordering == Some(std::cmp::Ordering::Equal)));
        }
        if op == "!=" {
            return Ok(Value::Bool(ordering != Some(std::cmp::Ordering::Equal)));
        }
        let cmp = ordering
            .ok_or_else(|| format!("无法比较 `{}` 和 `{}`", value_string(&a), value_string(&b)))?;
        return Ok(Value::Bool(match op {
            ">" => cmp.is_gt(),
            ">=" => !cmp.is_lt(),
            "<" => cmp.is_lt(),
            _ => !cmp.is_gt(),
        }));
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => match op {
            "+" => x.checked_add(y).map(Value::Int),
            "-" => x.checked_sub(y).map(Value::Int),
            "*" => x.checked_mul(y).map(Value::Int),
            "/" if y != 0 => x.checked_div(y).map(Value::Int),
            "%" if y != 0 => x.checked_rem(y).map(Value::Int),
            "/" | "%" => return Err("除数不能为零".into()),
            _ => None,
        }
        .ok_or_else(|| "整数运算溢出".to_owned()),
        (Value::Float(x), Value::Float(y)) => match op {
            "+" => Ok(Value::Float(x + y)),
            "-" => Ok(Value::Float(x - y)),
            "*" => Ok(Value::Float(x * y)),
            "/" if y != 0.0 => Ok(Value::Float(x / y)),
            "%" if y != 0.0 => Ok(Value::Float(x % y)),
            "/" | "%" => Err("除数不能为零".into()),
            _ => Err(format!("未知运算符 `{op}`")),
        },
        (a, b) => Err(format!(
            "运算符 `{op}` 不支持 `{}` 和 `{}`",
            value_string(&a),
            value_string(&b)
        )),
    }
}

fn default_member_value(annotation: Option<&str>) -> Value {
    match annotation {
        Some("int") => Value::Int(0),
        Some("float") => Value::Float(0.0),
        Some("string") => Value::String(String::new()),
        Some("bool") => Value::Bool(false),
        _ => Value::Null,
    }
}

fn value_type(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Int(_) => "int".into(),
        Value::Float(_) => "float".into(),
        Value::String(_) => "string".into(),
        Value::List(_) => "list".into(),
        Value::Dict(_) => "dict".into(),
        Value::Struct { name, .. } => name.clone(),
    }
}

fn member_value(value: &Value, field: &str) -> Result<Value, String> {
    let Value::Struct { name, fields } = value else {
        return Err("成员访问只能用于结构体".into());
    };
    fields
        .iter()
        .find(|member| member.name == field)
        .map(|member| member.value.clone())
        .ok_or_else(|| format!("结构体 `{name}` 没有成员 `{field}`"))
}

fn index_value(container: &Value, index: &Value) -> Result<Value, String> {
    match container {
        Value::List(values) => {
            let Value::Int(index) = index else {
                return Err("列表下标必须是 int".into());
            };
            if *index < 0 {
                return Ok(Value::Int(0));
            }
            Ok(values
                .get(*index as usize)
                .cloned()
                .unwrap_or(Value::Int(0)))
        }
        Value::Dict(entries) => {
            for (key, value) in entries {
                if matches!(binary(key.clone(), "==", index.clone())?, Value::Bool(true)) {
                    return Ok(value.clone());
                }
            }
            Ok(Value::Int(0))
        }
        _ => Err("下标访问只能用于列表或字典".into()),
    }
}

fn reject_container(value: &Value) -> Result<(), String> {
    if matches!(
        value,
        Value::List(_) | Value::Dict(_) | Value::Struct { .. }
    ) {
        Err("列表、字典和结构体不能进行类型转换".into())
    } else {
        Ok(())
    }
}

fn convert_int(value: Value) -> Result<Value, String> {
    reject_container(&value)?;
    match value {
        Value::Int(n) => Ok(Value::Int(n)),
        Value::Bool(b) => Ok(Value::Int(if b { 1 } else { 0 })),
        Value::Float(n) if n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64 => {
            Ok(Value::Int(n as i64))
        }
        Value::Float(_) => Err("float 超出 int 范围".into()),
        Value::String(s) => {
            if let Ok(n) = s.parse::<i64>() {
                Ok(Value::Int(n))
            } else {
                let n = s
                    .parse::<f64>()
                    .map_err(|_| "字符串无法转换为 int".to_owned())?;
                if n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                    Ok(Value::Int(n as i64))
                } else {
                    Err("字符串无法转换为 int".into())
                }
            }
        }
        Value::Null => Ok(Value::Int(0)),
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } => unreachable!(),
    }
}

fn convert_float(value: Value) -> Result<Value, String> {
    reject_container(&value)?;
    match value {
        Value::Float(n) => Ok(Value::Float(n)),
        Value::Int(n) => Ok(Value::Float(n as f64)),
        Value::Bool(b) => Ok(Value::Float(if b { 1.0 } else { 0.0 })),
        Value::String(s) => s
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| "字符串无法转换为 float".into()),
        Value::Null => Ok(Value::Float(0.0)),
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } => unreachable!(),
    }
}

fn convert_string(value: Value) -> Result<Value, String> {
    reject_container(&value)?;
    Ok(Value::String(value_string(&value)))
}

fn convert_bool(value: Value) -> Result<Value, String> {
    reject_container(&value)?;
    match value {
        Value::Bool(b) => Ok(Value::Bool(b)),
        Value::Int(n) => Ok(Value::Bool(n != 0)),
        Value::Float(n) => Ok(Value::Bool(n != 0.0)),
        Value::String(s) => Ok(Value::Bool(!s.is_empty())),
        Value::Null => Ok(Value::Bool(false)),
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } => unreachable!(),
    }
}

fn value_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(value_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Dict(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| format!("{}: {}", value_string(key), value_string(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Struct { name, fields } => format!(
            "{}{{{}}}",
            name,
            fields
                .iter()
                .map(|field| format!("{}: {}", field.name, value_string(&field.value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("用法：kyptc <源文件.khyept>\n示例：kyptc examples/basics.khyept");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("错误：一次只能运行一个源文件");
        return ExitCode::from(2);
    }
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("无法读取 `{path}`：{e}");
            return ExitCode::FAILURE;
        }
    };
    let result = lex(&source)
        .and_then(|tokens| Parser::new(tokens).program())
        .and_then(|ast| Interpreter::new().run(&ast));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("运行错误：{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_features() {
        let src = r#"let int fixed = 1; var int count = 2; fn int add(int a, int b) { return(a + b); } println(f"{fixed}{add(count, 3)}");"#;
        let tokens = lex(src).unwrap();
        assert!(Parser::new(tokens).program().is_ok());
    }

    #[test]
    fn parses_print_and_println_as_distinct_statements() {
        let ast = Parser::new(lex("print(\"first\"); println(\"second\");").unwrap())
            .program()
            .unwrap();

        assert!(matches!(
            ast.as_slice(),
            [Stmt::Print(Expr::Value(Value::String(first)), false),
             Stmt::Print(Expr::Value(Value::String(second)), true)]
                if first == "first" && second == "second"
        ));
    }

    #[test]
    fn evaluates_function_and_mutable_variable() {
        let src = "var int n = 1; fn int inc(int x) { return(x + 1); } n = inc(n); println(n);";
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
        assert!(matches!(interpreter.lookup("n"), Some(Value::Int(2))));
    }

    #[test]
    fn rejects_assignment_to_let() {
        let ast = Parser::new(lex("let int n = 1; n = 2;").unwrap())
            .program()
            .unwrap();
        assert!(Interpreter::new()
            .run(&ast)
            .unwrap_err()
            .contains("固定变量"));
    }

    #[test]
    fn parses_readln_call() {
        let ast = Parser::new(lex("let string name = readln(\"name: \" );").unwrap())
            .program()
            .unwrap();
        assert!(
            matches!(ast.first(), Some(Stmt::Var { value: Some(Expr::Call(name, _)), .. }) if name == "readln")
        );
    }

    #[test]
    fn evaluates_if_and_recursive_fibonacci() {
        let src = r#"
            fn int fibonacci(n) {
                if (n <= 0) { return(0); }
                elif (n == 1) { return(1); }
                else { return(fibonacci(n - 1) + fibonacci(n - 2)); }
            }
            let int result = if (fibonacci(5) == 5) { 10 } else { 20 };
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
        assert!(matches!(interpreter.lookup("result"), Some(Value::Int(10))));
    }

    #[test]
    fn evaluates_switch_cases_ranges_and_fallthrough() {
        let src = r#"
            var int marker = 0;
            switch (1) {
                case (1) { marker = marker + 1; }
                case (2) { marker = marker + 10; break; }
                default { marker = 100; }
            }
            switch ("Jack") {
                case ("Tom", "Jack", "Jackson") { marker = marker + 100; break; }
                default { marker = 1000; }
            }
            switch (7) {
                case (1...5) { marker = 2000; break; }
                case (5<..10) { marker = marker + 1000; break; }
                default { marker = 3000; }
            }
            switch (50) {
                case (10..<51) { marker = marker + 10000; break; }
                default { marker = 30000; }
            }
            switch (51) {
                case (10..<51) { marker = 40000; break; }
                default { marker = marker + 100000; }
            }
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
        assert!(matches!(
            interpreter.lookup("marker"),
            Some(Value::Int(111111))
        ));
    }

    #[test]
    fn supports_uninitialized_declaration_and_default() {
        let src =
            "let int n; var string result; switch (n) { default { result = \"default\"; break; } }";
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
        assert!(
            matches!(interpreter.lookup("result"), Some(Value::String(value)) if value == "default")
        );
    }

    #[test]
    fn decodes_string_escape_sequences() {
        let src = r#"
            let string value = "\n\t\\\'\"\r\b\f\v\a\0\101\x42\u4e2d\U0001F600";
            let string formatted = f"prefix\\n{value}";
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("value"),
            Some(Value::String(value))
                if value == "\n\t\\'\"\r\u{0008}\u{000c}\u{000b}\u{0007}\0AB中😀"
        ));
        assert!(matches!(
            interpreter.lookup("formatted"),
            Some(Value::String(value)) if value == "prefix\\n\n\t\\'\"\r\u{0008}\u{000c}\u{000b}\u{0007}\0AB中😀"
        ));
    }

    #[test]
    fn rejects_invalid_string_escape_sequences() {
        for source in [
            r#"let string value = "\q";"#,
            r#"let string value = "\x4";"#,
            r#"let string value = "\u12xz";"#,
            r#"let string value = "\U00110000";"#,
        ] {
            assert!(lex(source).is_err(), "expected invalid escape: {source}");
        }
    }

    #[test]
    fn evaluates_conversions_collections_and_indexing() {
        let src = r#"
            var int a = 123;
            var string initial_type = f"{type(a)}";
            a = string(a);
            a += ".67";
            var string string_value = f"{type(a)} a = {a}";
            a = float(a);
            var string float_value = f"{type(a)} a = {a}";
            a = bool(int(a - 0.67));
            var string bool_value = f"{type(a)} a = {a}";
            var l = [1, 2, 3, "4"];
            var d = {1: 1, "2": "false", 3: "3", "4": true};
            var int list_value = l[2];
            var string dict_value = d[string(2)];
            var int missing = l[99] + d["missing"];
            var int list_size = size(l);
            var int dict_size = size(d);
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("a"), Some(Value::Bool(true))));
        assert!(
            matches!(interpreter.lookup("initial_type"), Some(Value::String(value)) if value == "int")
        );
        assert!(
            matches!(interpreter.lookup("string_value"), Some(Value::String(value)) if value == "string a = 123.67")
        );
        assert!(
            matches!(interpreter.lookup("float_value"), Some(Value::String(value)) if value == "float a = 123.67")
        );
        assert!(
            matches!(interpreter.lookup("bool_value"), Some(Value::String(value)) if value == "bool a = true")
        );
        assert!(matches!(
            interpreter.lookup("list_value"),
            Some(Value::Int(3))
        ));
        assert!(
            matches!(interpreter.lookup("dict_value"), Some(Value::String(value)) if value == "false")
        );
        assert!(matches!(interpreter.lookup("missing"), Some(Value::Int(0))));
        assert!(matches!(
            interpreter.lookup("list_size"),
            Some(Value::Int(4))
        ));
        assert!(matches!(
            interpreter.lookup("dict_size"),
            Some(Value::Int(4))
        ));
    }

    #[test]
    fn evaluates_for_while_and_do_while_loops() {
        let src = r#"
            var int total = 0;
            for (var int i = 0; i <= 4; i++) { total += i; }
            for (total = 15; total < 17; total++) { }
            var int j = 0;
            while (j < 3) { total += ++j; }
            var int k = 0;
            do { total += k++; } while (k < 3);
            var string post = f"{k++}";
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("total"), Some(Value::Int(26))));
        assert!(matches!(interpreter.lookup("post"), Some(Value::String(value)) if value == "3"));
        assert!(matches!(interpreter.lookup("k"), Some(Value::Int(4))));
    }

    #[test]
    fn evaluates_struct_members_and_type() {
        let src = r#"
            struct Student {
                let int id;
                var int age;
                var string name;
            };
            var Student s{1, 15, "Jack"};
            var string first = f"{s.id}\n{s.age}\n{s.name}";
            var string student_type = type(s);
            s.age = 16;
            s.name = "Tom";
            var string second = f"{s.id}\n{s.age}\n{s.name}";
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("first"),
            Some(Value::String(value)) if value == "1\n15\nJack"
        ));
        assert!(matches!(
            interpreter.lookup("second"),
            Some(Value::String(value)) if value == "1\n16\nTom"
        ));
        assert!(matches!(
            interpreter.lookup("student_type"),
            Some(Value::String(value)) if value == "Student"
        ));
    }

    #[test]
    fn evaluates_struct_collection_members_and_missing_indices() {
        let src = r#"
            struct Node { var lis; var dic; };
            var Node h{[3, 2, 3, 4, 7], {1: 1, "2": 2}};
            var int first = h.lis[0];
            var int second = h.dic["2"];
            var int missing = h.lis[8];
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("first"), Some(Value::Int(3))));
        assert!(matches!(interpreter.lookup("second"), Some(Value::Int(2))));
        assert!(matches!(interpreter.lookup("missing"), Some(Value::Int(0))));
    }

    #[test]
    fn rejects_assignment_to_fixed_struct_member() {
        let src = r#"
            struct Student { let int id; var int age; };
            var Student s{1, 2};
            s.id = 3;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let error = Interpreter::new().run(&ast).unwrap_err();
        assert!(error.contains("固定成员"));
    }

    #[test]
    fn declares_struct_instance_and_uses_zero_defaults() {
        let src = r#"
            struct student {
                var int id;
                var int age;
                var string name;
            } s;
            var string before = f"{s.id}|{s.age}|{s.name}";
            s.id = 7;
            s.age = 8;
            s.name = "Ann";
            var string after = f"{s.id}|{s.age}|{s.name}";
            var string student_type = type(s);
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("before"),
            Some(Value::String(value)) if value == "0|0|"
        ));
        assert!(matches!(
            interpreter.lookup("after"),
            Some(Value::String(value)) if value == "7|8|Ann"
        ));
        assert!(matches!(
            interpreter.lookup("student_type"),
            Some(Value::String(value)) if value == "student"
        ));
    }

    #[test]
    fn accesses_struct_instance_from_function() {
        let src = r#"
            struct student {
                var int id;
                var int age;
                var string name;
            } s;
            fn int main() {
                s.id = 7;
                s.age = 8;
                s.name = "Ann";
                return(s.id + s.age);
            }
            var int result = main();
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("result"), Some(Value::Int(15))));
        assert!(matches!(
            interpreter.lookup("s"),
            Some(Value::Struct { fields, .. })
                if matches!(&fields[0].value, Value::Int(7))
                    && matches!(&fields[1].value, Value::Int(8))
                    && matches!(&fields[2].value, Value::String(name) if name == "Ann")
        ));
    }

    #[test]
    fn declares_struct_instances_with_initializers_and_lists() {
        let src = r#"
            struct Point { var int x; var int y; } p1, p2;
            p1.x = 3;
            p2.y = 9;
            struct Box { var int w; var string tag; } b{5, "box"};
            struct wrap { var int a; } w{1};
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("p1"),
            Some(Value::Struct { name, fields })
                if name == "Point"
                    && matches!(&fields[0].value, Value::Int(3))
                    && matches!(&fields[1].value, Value::Int(0))
        ));
        assert!(matches!(
            interpreter.lookup("p2"),
            Some(Value::Struct { fields, .. })
                if matches!(&fields[0].value, Value::Int(0))
                    && matches!(&fields[1].value, Value::Int(9))
        ));
        assert!(matches!(
            interpreter.lookup("b"),
            Some(Value::Struct { fields, .. })
                if matches!(&fields[0].value, Value::Int(5))
                    && matches!(&fields[1].value, Value::String(tag) if tag == "box")
        ));
    }

    #[test]
    fn requires_semicolon_after_struct_instance_declaration() {
        let error = Parser::new(lex("struct a { var int x; } s").unwrap())
            .program()
            .unwrap_err();
        assert!(error.contains("`;`"), "unexpected error: {error}");
    }
}
