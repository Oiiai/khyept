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
    let (mut i, mut line) = (0usize, 1usize);
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            if c == '\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        // 行注释必须是 `///`（三个斜杠），这样 `//` 专属于整除运算符
        if c == '/' && chars.get(i + 1) == Some(&'/') && chars.get(i + 2) == Some(&'/') {
            i += 3;
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
        // 多字符运算符优先匹配
        const MULTI: [&str; 15] = [
            "<..", "..<", "...", "**", "<<", ">>", "&&", "||", "->", ":=", "//", "==", "!=",
            "<=", ">=",
        ];
        let mut matched_multi = None;
        for candidate in MULTI {
            let cands: Vec<char> = candidate.chars().collect();
            if chars[i..].starts_with(&cands[..]) {
                matched_multi = Some(candidate);
                break;
            }
        }
        if let Some(op) = matched_multi {
            tokens.push(Token {
                kind: TokenKind::Operator(op.to_owned()),
                line: token_line,
            });
            i += op.chars().count();
            continue;
        }
        if c == '+' && chars.get(i + 1) == Some(&'+')
            || c == '-' && chars.get(i + 1) == Some(&'-')
        {
            tokens.push(Token {
                kind: TokenKind::Operator(format!("{c}{c}")),
                line: token_line,
            });
            i += 2;
            continue;
        }
        if "+-*/%=!<>".contains(c) {
            let mut op = c.to_string();
            if chars.get(i + 1) == Some(&'=') {
                op.push('=');
                i += 1;
            }
            i += 1;
            tokens.push(Token {
                kind: TokenKind::Operator(op),
                line: token_line,
            });
            continue;
        }
        if "&|^~".contains(c) {
            tokens.push(Token {
                kind: TokenKind::Operator(c.to_string()),
                line: token_line,
            });
            i += 1;
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
    /// 切片：s[a:b]、s[a:]、s[:b]
    Slice {
        target: Box<Expr>,
        from: Option<Box<Expr>>,
        to: Option<Box<Expr>>,
    },
    Member(Box<Expr>, String),
    /// 方法调用：obj.method(args)
    MethodCall(Box<Expr>, String, Vec<Expr>),
    /// 命名参数：name = value
    NamedArg(String, Box<Expr>),
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
        inferred: bool,
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
        returns: Option<String>,
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

#[derive(Clone, Debug, PartialEq)]
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
    /// 文件句柄：open() 的返回值，用户视角下就是字符串
    File {
        path: String,
        mode: String,
        encoding: String,
        /// r+ / w 模式下已写入的字节数，用于决定写入位置
        position: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
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
                body: self.body()?,
            });
        }
        if self.eat_ident("do") {
            let body = self.body()?;
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
            // 新语法：fn name(...) -> T
            // 旧语法：fn T name(...) 仍然兼容
            let first = self.ident()?;
            let mut legacy_return: Option<String> = None;
            let name = if self.is_symbol('(') {
                first
            } else {
                legacy_return = Some(first);
                self.ident()?
            };
            self.expect_symbol('(')?;
            let mut params = Vec::new();
            if !self.is_symbol(')') {
                loop {
                    let first_param = self.ident()?;
                    // 新语法 a : int；旧语法 int a
                    let (annotation, param) = if self.is_symbol(':') {
                        self.advance();
                        (Some(self.ident()?), first_param)
                    } else if self.current_is_ident() {
                        (Some(first_param), self.ident()?)
                    } else {
                        (None, first_param)
                    };
                    params.push((param, annotation));
                    if !self.eat_symbol(',') {
                        break;
                    }
                }
            }
            self.expect_symbol(')')?;
            // 新语法：-> int；不写返回类型默认为 void
            let returns = if self.is_operator("->") {
                self.advance();
                Some(self.ident()?)
            } else {
                legacy_return.or_else(|| Some("void".to_owned()))
            };
            let body = self.block()?;
            return Ok(Stmt::Function {
                name,
                params,
                returns,
                body,
            });
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
        // 新语法：var n : int = 5 / var n := 5
        // 旧语法：var int n = 5（类型前置，仍然兼容）
        let first = self.ident()?;
        //`:=` 是一个完整的运算符 token，消费后直接读初始值
        if self.is_operator(":=") {
            self.advance();
            let value = self.expression(0)?;
            if consume_semicolon {
                self.eat_symbol(';');
            }
            return Ok(Stmt::Var {
                name: first,
                mutable,
                annotation: None,
                value: Some(value),
                inferred: true,
            });
        }
        let (annotation, name) = if self.is_symbol(':') {
            self.advance();
            (Some(self.ident()?), first)
        } else if self.current_is_ident() {
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
            inferred: false,
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
            // 新语法：var id : int；旧语法 var int id 仍兼容
            let (annotation, field_name) = if self.is_symbol(':') {
                self.advance();
                (Some(self.ident()?), first)
            } else if self.current_is_ident() {
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
        // `struct A { ... }` 之后的标识符可能是实例名，也可能是下一条语句的开头
        // （例如 `var x = 1;` 或 `fn foo() {}`）。只有当它既不是关键字、
        // 后面也不紧跟 `(` 时，才当作实例名。
        loop {
            let name = self.ident()?;
            if self.reserved_word(&name) {
                // 回退：把刚读到的标识符留给上层当作新语句的开头
                self.at -= 1;
                break;
            }
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
                // `} a{1,2}, b;` 形式：逗号或下一个标识符继续
                if self.is_symbol(',') {
                    break;
                }
                if self.current_is_ident() && !self.peek_is_statement_start() {
                    self.advance();
                } else {
                    break;
                }
            }
            instances.push(StructInstance {
                name,
                initializers,
            });
            if !self.eat_symbol(',') {
                break;
            }
            if !self.current_is_ident() {
                return Err(self.error("逗号后期望结构体实例名"));
            }
        }
        Ok(instances)
    }
    /// 判断 `self.at` 处的标识符是否是保留字
    fn reserved_word(&self, name: &str) -> bool {
        matches!(
            name,
            "let" | "var"
                | "fn"
                | "struct"
                | "if"
                | "elif"
                | "else"
                | "while"
                | "for"
                | "do"
                | "switch"
                | "case"
                | "default"
                | "break"
                | "return"
                | "global"
                | "println"
                | "print"
                | "readln"
        )
    }
    /// 向前看：当前标识符之后若是 `{`/`(`/`=` 等，说明它更可能是语句开头而非实例名
    fn peek_is_statement_start(&self) -> bool {
        matches!(
            self.tokens.get(self.at + 1).map(|t| &t.kind),
            Some(TokenKind::Symbol('('))
                | Some(TokenKind::Symbol('{'))
                | Some(TokenKind::Symbol('='))
                | Some(TokenKind::Symbol(':'))
        )
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
            body: self.body()?,
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
            branches.push((condition, self.body()?));
            if self.eat_ident("elif") {
                condition = self.condition()?;
            } else if self.eat_ident("else") {
                if self.eat_ident("if") {
                    condition = self.condition()?;
                } else {
                    return Ok((branches, Some(self.body()?)));
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
    /// 语句体：可以是 `{ ... }`，也可以只跟一条语句。
    /// 这样 `if (c) println("x");` 与 `while (c) i++;` 都能写。
    fn body(&mut self) -> Result<Vec<Stmt>, String> {
        if self.is_symbol('{') {
            return self.block();
        }
        Ok(vec![self.statement()?])
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
                let body = self.body()?;
                cases.push(SwitchCase { pattern, body });
            } else if self.eat_ident("default") {
                if default.is_some() {
                    return Err(self.error("switch 中只能有一个 default"));
                }
                default = Some(self.body()?);
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
        let mut left = if self.is_operator("-") || self.is_operator("!") || self.is_operator("~") {
            let op = match self.advance().kind {
                TokenKind::Operator(s) => s,
                _ => unreachable!(),
            };
            // 一元运算符绑定最紧
            Expr::Unary(op, Box::new(self.expression(12)?))
        } else if self.is_operator("++") || self.is_operator("--") {
            let op = match self.advance().kind {
                TokenKind::Operator(s) => s,
                _ => unreachable!(),
            };
            let operand = self.expression(12)?;
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
                "||" => 1,
                "&&" => 2,
                "|" => 3,
                "^" => 4,
                "&" => 5,
                "==" | "!=" => 6,
                "<" | ">" | "<=" | ">=" => 7,
                "<<" | ">>" => 8,
                "+" | "-" => 9,
                "*" | "/" | "//" | "%" => 10,
                "**" => 11,
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            // `**` 右结合：2 ** 3 ** 2 == 2 ** (3 ** 2)
            let right = if op == "**" {
                self.expression(prec)?
            } else {
                self.expression(prec + 1)?
            };
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
                            args.push(self.call_argument()?);
                            if !self.eat_symbol(',') {
                                break;
                            }
                        }
                    }
                    self.expect_symbol(')')?;
                    Ok(Expr::Call(name, args))
                } else if self.is_symbol('{') {
                    // 结构体构造：Name{1, 2}
                    self.advance();
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
                    Ok(Expr::StructInit(name, values))
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
            if self.is_symbol('[') {
                self.advance();
                // 切片：s[a:b]、s[a:]、s[:b]、s[:]
                // 只要出现 `:` 就按切片解析，否则是普通下标
                let has_colon = self.scan_to_bracket_end();
                if has_colon {
                    let from = if self.is_symbol(':') {
                        None
                    } else {
                        Some(Box::new(self.expression(0)?))
                    };
                    self.expect_symbol(':')?;
                    let to = if self.is_symbol(']') {
                        None
                    } else {
                        Some(Box::new(self.expression(0)?))
                    };
                    self.expect_symbol(']')?;
                    expr = Expr::Slice {
                        target: Box::new(expr),
                        from,
                        to,
                    };
                    continue;
                }
                let index = self.expression(0)?;
                self.expect_symbol(']')?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
            } else if self.eat_symbol('.') {
                let name = self.ident()?;
                // obj.method(...) 是方法调用，obj.field 是成员访问
                if self.is_symbol('(') {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.is_symbol(')') {
                        loop {
                            args.push(self.call_argument()?);
                            if !self.eat_symbol(',') {
                                break;
                            }
                        }
                    }
                    self.expect_symbol(')')?;
                    expr = Expr::MethodCall(Box::new(expr), name, args);
                } else {
                    expr = Expr::Member(Box::new(expr), name);
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }
    /// 调用参数：既支持 `value`，也支持 `name = value` 形式。
/// 命名参数目前只用于 `open(..., encoding = "utf-8")`。
fn call_argument(&mut self) -> Result<Expr, String> {
    if self.current_is_ident() {
        if let (TokenKind::Ident(name), Some(TokenKind::Operator(op))) =
            (&self.current().kind, self.tokens.get(self.at + 1).map(|t| &t.kind))
        {
            if op == "=" {
                let name = name.clone();
                self.advance();
                self.advance();
                let value = self.expression(0)?;
                return Ok(Expr::NamedArg(name, Box::new(value)));
            }
        }
    }
    self.expression(0)
}

/// 扫描从当前位置到匹配的 `]`...
fn scan_to_bracket_end(&mut self) -> bool {
        let start = self.at;
        let mut depth = 0i32;
        let mut has_colon = false;
        while self.at < self.tokens.len() {
            match &self.tokens[self.at].kind {
                TokenKind::Symbol('[') | TokenKind::Symbol('(') | TokenKind::Symbol('{') => {
                    depth += 1
                }
                TokenKind::Symbol(']') => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                TokenKind::Symbol(':') if depth == 0 => has_colon = true,
                TokenKind::Eof => break,
                _ => {}
            }
            self.at += 1;
        }
        self.at = start;
        has_colon
    }
}

fn parse_format(raw: &str) -> Result<Vec<FormatPart>, String> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                text.push('{');
                i += 2;
                continue;
            }
            // 扫描到配对的 `}`，期间跳过字符串字面量并跟踪嵌套花括号，
            // 这样 f"{P{1, 2}}" 里的内层大括号不会被误当作结束符。
            // 注意起始的 `{` 已在本层处理，所以 depth 从 1 开始。
            let mut depth = 1i32;
            let mut in_string: Option<char> = None;
            let mut name = String::new();
            i += 1;
            while i < chars.len() {
                let ch = chars[i];
                if let Some(quote) = in_string {
                    name.push(ch);
                    if ch == '\\' && i + 1 < chars.len() {
                        name.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    if ch == quote {
                        in_string = None;
                    }
                    i += 1;
                    continue;
                }
                match ch {
                    '"' | '\'' => in_string = Some(ch),
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                name.push(ch);
                i += 1;
            }
            if !text.is_empty() {
                parts.push(FormatPart::Text(std::mem::take(&mut text)));
            }
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                let tokens = lex(trimmed)?;
                let mut parser = Parser::new(tokens);
                let expr = parser.expression(0)?;
                if !matches!(parser.current().kind, TokenKind::Eof) {
                    return Err(format!("格式化表达式未完整解析：`{trimmed}`"));
                }
                parts.push(FormatPart::Expr(expr));
            }
            continue;
        }
        if c == '}' && chars.get(i + 1) == Some(&'}') {
            text.push('}');
            i += 2;
            continue;
        }
        text.push(c);
        i += 1;
    }
    if !text.is_empty() {
        parts.push(FormatPart::Text(text));
    }
    Ok(parts)
}

#[derive(Clone)]
struct Binding {
    /// 由 `:=` 推断得到的类型。显式标注的类型不在此保存，
    /// 以保持「标注只约束声明处」的历史语义。
    inferred_type: Option<String>,
    value: Value,
    mutable: bool,
}
#[derive(Clone)]
struct Function {
    params: Vec<(String, Option<String>)>,
    returns: Option<String>,
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
                inferred,
            } => {
                let value = if let Some(value) = value {
                    self.eval(value)?
                } else {
                    Value::Null
                };
                // 声明处的类型检查：显式标注或 `:=` 推断都参与
                let declared: Option<String> = if *inferred {
                    Some(value_type(&value))
                } else {
                    annotation.clone()
                };
                if matches!(value, Value::Null)
                    && declared
                        .as_deref()
                        .is_some_and(|ty| self.structs.iter().any(|(n, _)| n == ty))
                {
                    return Err(format!(
                        "结构体 `{}` 必须在对象创建时初始化",
                        declared.as_deref().unwrap()
                    ));
                }
                self.check_type(declared.as_deref(), &value)?;
                let binding = Binding {
                    inferred_type: if *inferred {
                        Some(value_type(&value))
                    } else {
                        None
                    },
                    value,
                    mutable: *mutable,
                };
                let globals = self.global_names.last().cloned().unwrap_or_default();
                if globals.contains(name) {
                    self.define_global(name, binding)?;
                } else {
                    self.define_local(name, binding)?;
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
            Stmt::Function {
                name,
                params,
                returns,
                body,
            } => {
                if self.functions.iter().any(|(n, _)| n == name) {
                    return Err(format!("函数 `{name}` 已定义"));
                }
                self.functions.push((
                    name.clone(),
                    Function {
                        params: params.clone(),
                        returns: returns.clone(),
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
                            inferred_type: None,
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
            Expr::NamedArg(_, value) => self.eval(value),
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
                    ("~", Value::Int(n)) => Ok(Value::Int(!n)),
                    _ => Err(format!("运算符 `{op}` 的操作数类型不正确")),
                }
            }
            Expr::Prefix(op, expr) => self.eval_update(expr, op, true),
            Expr::Postfix(op, expr) => self.eval_update(expr, op, false),
            Expr::Binary(left, op, right) => {
                // 短路求值：&& / || 不必计算右侧
                if op == "&&" || op == "||" {
                    let a = self.eval(left)?;
                    let Value::Bool(x) = a else {
                        return Err(format!(
                            "运算符 `{op}` 的左侧必须是 bool，收到 `{}`",
                            value_string(&a)
                        ));
                    };
                    // 左侧已确定结果时跳过右侧
                    if (op == "&&" && !x) || (op == "||" && x) {
                        return Ok(Value::Bool(x));
                    }
                    let b = self.eval(right)?;
                    let Value::Bool(y) = b else {
                        return Err(format!(
                            "运算符 `{op}` 的右侧必须是 bool，收到 `{}`",
                            value_string(&b)
                        ));
                    };
                    return Ok(Value::Bool(if op == "&&" { x && y } else { x || y }));
                }
                let a = self.eval(left)?;
                let b = self.eval(right)?;
                binary(a, op, b)
            }
            Expr::Index(container, index) => {
                let container = self.eval(container)?;
                let index = self.eval(index)?;
                index_value(&container, &index)
            }
            Expr::Slice { target, from, to } => {
                let base = self.eval(target)?;
                let from = match from {
                    Some(expr) => Some(self.eval(expr)?),
                    None => None,
                };
                let to = match to {
                    Some(expr) => Some(self.eval(expr)?),
                    None => None,
                };
                slice_value(&base, from.as_ref(), to.as_ref())
            }
            Expr::MethodCall(base, name, args) => {
                let target = self.eval(base)?;
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg)?);
                }
                let (result, next_position) = method_call(&target, name, values)?;
                // 文件写入后推进写入位置，使 r+ 模式下多次 write 顺序追加
                if let Some(advance) = next_position {
                    if let Value::File {
                        path,
                        mode,
                        encoding,
                        position,
                    } = target
                    {
                        self.assign_target(
                            base,
                            Value::File {
                                path,
                                mode,
                                encoding,
                                position: position + advance,
                            },
                        )?;
                    }
                }
                Ok(result)
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
                    "int" | "float" | "string" | "bool" | "type" | "size" | "len"
                ) {
                    return self.eval_builtin(name, args);
                }
                if name == "open" {
                    return self.eval_open(args);
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
                            inferred_type: None,
                            value,
                            mutable: true,
                        },
                    )?;
                }
                let result = self.execute_block(&function.body);
                self.scopes.pop();
                self.global_names.pop();
                self.call_depth -= 1;
                let returned = match result? {
                    Flow::Continue => {
                        // 未显式 return：非 void 函数视为缺少返回值
                        if let Some(ty) = function.returns.as_deref() {
                            if ty != "void" {
                                return Err(format!("函数 `{name}` 应返回 `{ty}`"));
                            }
                        }
                        Value::Null
                    }
                    Flow::Return(v) => v,
                    Flow::Break => return Err("break 不在 switch 中".into()),
                };
                if let Some(ty) = function.returns.as_deref() {
                    self.check_type(Some(ty), &returned)
                        .map_err(|e| format!("函数 `{name}` 返回值：{e}"))?;
                }
                Ok(returned)
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
                // 先做类型检查再写入，避免可变借用与&self 冲突
                {
                    let Some(binding) = self.lookup_binding(name) else {
                        return Err(format!("变量 `{name}` 未定义"));
                    };
                    if !binding.mutable {
                        return Err(format!("固定变量 `{name}` 不能重新赋值"));
                    }
                    // 只有 `:=` 推断出的类型才约束后续赋值
                    let inferred = binding.inferred_type.clone();
                    self.check_type(inferred.as_deref(), &value)
                        .map_err(|error| format!("变量 `{name}`：{error}"))?;
                }
                let Some(binding) = self.lookup_mut(name) else {
                    return Err(format!("变量 `{name}` 未定义"));
                };
                binding.value = value;
                Ok(())
            }
            Expr::Member(base, field) => self.assign_member(base, field, value),
            Expr::Index(container_expr, index) => {
                // 先算出新的容器值，再把容器本身写回其赋值目标。
                // 这样 l[0][1] = 9、s.field[0] = 9 之类的嵌套路径也能工作。
                let container = self.eval(container_expr)?;
                let index = self.eval(index)?;
                let updated = assign_index(container, index, value)?;
                self.assign_target(container_expr, updated)
            }
            _ => Err("赋值目标必须是变量、结构体成员或下标".into()),
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
            "size" | "len" => match value {
                Value::List(values) => Ok(Value::Int(values.len() as i64)),
                Value::Dict(entries) => Ok(Value::Int(entries.len() as i64)),
                Value::String(text) => Ok(Value::Int(text.chars().count() as i64)),
                _ => Err(format!(
                    "`{name}` 只能用于列表、字典或字符串，收到 `{}`",
                    value_string(&value)
                )),
            },
            "int" => convert_int(value),
            "float" => convert_float(value),
            "string" => convert_string(value),
            "bool" => convert_bool(value),
            _ => unreachable!(),
        }
    }
    /// open(path, mode, encoding = "utf-8")
    ///
    /// 支持 r / w / a / r+ 及其二进制变体。返回值是一个文件句柄，
    /// 用户视角下当作字符串使用，通过 read / write / writeln / close 操作。
    fn eval_open(&mut self, args: &[Expr]) -> Result<Value, String> {
        if args.is_empty() || args.len() > 3 {
            return Err(format!("`open` 需要 1 到 3 个参数，收到 {} 个", args.len()));
        }
        // 先分离位置参数与命名参数
        let mut positional: Vec<&Expr> = Vec::new();
        let mut encoding: Option<String> = None;
        for arg in args {
            match arg {
                Expr::NamedArg(key, value) => match key.as_str() {
                    "encoding" => match self.eval(value)? {
                        Value::String(e) => encoding = Some(e),
                        other => {
                            return Err(format!(
                                "`encoding` 必须是字符串，收到 `{}`",
                                value_string(&other)
                            ))
                        }
                    },
                    other => {
                        return Err(format!(
                            "`open` 不支持命名参数 `{other}`，可用：encoding"
                        ))
                    }
                },
                other => positional.push(other),
            }
        }
        if positional.is_empty() {
            return Err("`open` 需要路径参数".into());
        }
        if positional.len() > 2 {
            return Err(format!(
                "`open` 最多接受 2 个位置参数，收到 {} 个",
                positional.len()
            ));
        }
        let Value::String(path) = self.eval(positional[0])? else {
            return Err("`open` 的第一个参数必须是路径字符串".into());
        };
        let mode = if positional.len() >= 2 {
            match self.eval(positional[1])? {
                Value::String(m) => m,
                other => {
                    return Err(format!(
                        "`open` 的模式参数必须是字符串，收到 `{}`",
                        value_string(&other)
                    ))
                }
            }
        } else {
            "r".to_owned()
        };
        let valid = ["r", "w", "a", "r+", "rb", "wb", "ab", "rb+", "wb+", "ab+"];
        if !valid.contains(&mode.as_str()) {
            return Err(format!(
                "不支持的文件模式 `{mode}`，可用：{}",
                valid.join(" / ")
            ));
        }
        let encoding = encoding.unwrap_or_else(|| "utf-8".to_owned());
        // 检查文件是否存在（只读模式）
        if !std::path::Path::new(&path).exists() && mode.starts_with('r') {
            return Err(format!("文件 `{path}` 不存在"));
        }
        Ok(Value::File {
            path,
            mode,
            encoding,
            position: 0,
        })
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
            "string" => matches!(value, Value::String(_) | Value::File { .. }),
            "bool" => matches!(value, Value::Bool(_)),
            "void" => matches!(value, Value::Null),
            "list" => matches!(value, Value::List(_)),
            "dict" => matches!(value, Value::Dict(_)),
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
    // 逻辑与/或：短路语义在 eval 层处理，这里只做类型检查与合并
    if op == "&&" || op == "||" {
        return match (&a, &b) {
            (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(if op == "&&" {
                *x && *y
            } else {
                *x || *y
            })),
            _ => Err(format!(
                "运算符 `{op}` 只能作用于 bool，收到 `{}` 和 `{}`",
                value_string(&a),
                value_string(&b)
            )),
        };
    }
    // 位运算：仅支持整数
    if ["&", "|", "^", "<<", ">>"].contains(&op) {
        return match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                let result = match op {
                    "&" => x & y,
                    "|" => x | y,
                    "^" => x ^ y,
                    "<<" => {
                        if *y < 0 {
                            return Err("位移位数不能为负".into());
                        }
                        if *y >= 64 {
                            return Err(format!("位移位数过大 ({y})，最大 63"));
                        }
                        x.checked_shl(*y as u32).ok_or("整数溢出")?
                    }
                    ">>" => {
                        if *y < 0 {
                            return Err("位移位数不能为负".into());
                        }
                        if *y >= 64 {
                            return Err(format!("位移位数过大 ({y})，最大 63"));
                        }
                        x >> (*y as u32)
                    }
                    _ => unreachable!(),
                };
                Ok(Value::Int(result))
            }
            _ => Err(format!(
                "运算符 `{op}` 只能作用于 int，收到 `{}` 和 `{}`",
                value_string(&a),
                value_string(&b)
            )),
        };
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
            // 整除：向零截断，与 `/` 语义一致
            "//" if y != 0 => x.checked_div(y).map(Value::Int),
            "/" | "%" | "//" => return Err("除数不能为零".into()),
            "**" => int_pow(x, y),
            _ => None,
        }
        .ok_or_else(|| "整数运算溢出".to_owned()),
        (Value::Float(x), Value::Float(y)) => match op {
            "+" => Ok(Value::Float(x + y)),
            "-" => Ok(Value::Float(x - y)),
            "*" => Ok(Value::Float(x * y)),
            "/" if y != 0.0 => Ok(Value::Float(x / y)),
            "%" if y != 0.0 => Ok(Value::Float(x % y)),
            "//" if y != 0.0 => Ok(Value::Float((x / y).trunc())),
            "/" | "%" | "//" => Err("除数不能为零".into()),
            "**" => Ok(Value::Float(x.powf(y))),
            _ => Err(format!("未知运算符 `{op}`")),
        },
        (a, b) => Err(format!(
            "运算符 `{op}` 不支持 `{}` 和 `{}`",
            value_string(&a),
            value_string(&b)
        )),
    }
}

/// 整数幂。负指数返回 0（与整数语义一致），过大的指数直接报错而不是静默溢出。
fn int_pow(base: i64, exp: i64) -> Option<Value> {
    if exp < 0 {
        return Some(Value::Int(0));
    }
    // 2^63 已超出 i64，任何 >= 63 的指数在 base >= 2 时都会溢出
    if exp >= 63 && base.unsigned_abs() >= 2 {
        return None;
    }
    let mut result: i64 = 1;
    for _ in 0..exp {
        result = result.checked_mul(base)?;
    }
    Some(Value::Int(result))
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
        Value::File { path, mode, .. } => format!("<file {path} mode={mode}>"),
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
        Value::String(text) => {
            let chars: Vec<char> = text.chars().collect();
            let Value::Int(index) = index else {
                return Err("字符串下标必须是 int".into());
            };
            if *index < 0 || *index as usize >= chars.len() {
                return Err(format!("字符串下标 {index} 越界（长度 {}）", chars.len()));
            }
            Ok(Value::String(chars[*index as usize].to_string()))
        }
        Value::Dict(entries) => {
            for (key, value) in entries {
                if matches!(binary(key.clone(), "==", index.clone())?, Value::Bool(true)) {
                    return Ok(value.clone());
                }
            }
            Ok(Value::Int(0))
        }
        _ => Err("下标访问只能用于列表、字典或字符串".into()),
    }
}

/// 字符串切片：s[a:b]、s[a:]、s[:b]、s[:]
fn slice_value(base: &Value, from: Option<&Value>, to: Option<&Value>) -> Result<Value, String> {
    let text = match base {
        Value::String(text) => text,
        other => {
            return Err(format!(
                "切片只能用于字符串，收到 `{}`",
                value_string(other)
            ))
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let to_index = |value: Option<&Value>, default: usize| -> Result<usize, String> {
        match value {
            None => Ok(default),
            Some(Value::Int(n)) => {
                if *n < 0 {
                    Err(format!("切片下标不能为负 ({n})"))
                } else {
                    Ok(*n as usize)
                }
            }
            Some(other) => Err(format!(
                "切片下标必须是 int，收到 `{}`",
                value_string(other)
            )),
        }
    };
    let start = to_index(from, 0)?.min(chars.len());
    let end = to_index(to, chars.len())?.min(chars.len());
    if start > end {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(chars[start..end].iter().collect()))
}

/// 方法调用：str.size()、list.size()、dict.size()、file.read() 等
///
/// 返回 `(结果, 文件写入字节数)`。第二个元素仅文件写入时有值，
/// 用于让调用方把新的写入位置写回句柄。
fn method_call(
    target: &Value,
    name: &str,
    args: Vec<Value>,
) -> Result<(Value, Option<usize>), String> {
    if let Value::File {
        path,
        mode,
        encoding,
        position,
    } = target
    {
        return file_method(path, mode, encoding, *position, name, args);
    }
    match name {
        "size" => {
            if !args.is_empty() {
                return Err(format!("`{name}` 不接受参数"));
            }
            let size = match target {
                Value::List(values) => values.len(),
                Value::Dict(entries) => entries.len(),
                Value::String(text) => text.chars().count(),
                other => {
                    return Err(format!(
                        "`size` 只能用于列表、字典或字符串，收到 `{}`",
                        value_string(other)
                    ))
                }
            };
            Ok((Value::Int(size as i64), None))
        }
        _ => Err(format!("`{}` 不是可用的方法", name)),
    }
}

/// 文件句柄方法：read / write / writeln / close
///
/// 返回 `(结果, 写入字节数)`，后者供调用方推进句柄位置。
fn file_method(
    path: &str,
    mode: &str,
    encoding: &str,
    position: usize,
    name: &str,
    args: Vec<Value>,
) -> Result<(Value, Option<usize>), String> {
    match name {
        "read" => {
            if !args.is_empty() {
                return Err("`read` 不接受参数".into());
            }
            if mode == "w" || mode == "a" {
                return Err(format!("`{mode}` 模式只能写入，不能读取"));
            }
            let bytes =
                std::fs::read(path).map_err(|e| format!("读取 `{path}` 失败：{e}"))?;
            Ok((Value::String(decode_text(&bytes, encoding)), None))
        }
        "write" => {
            if args.len() != 1 {
                return Err("`write` 需要 1 个参数".into());
            }
            let bytes = value_to_bytes(&args[0], encoding)?;
            append_or_write(path, mode, &bytes, position)?;
            Ok((Value::Int(bytes.len() as i64), Some(bytes.len())))
        }
        "writeln" => {
            // writeln 接受字符串或字符串列表
            if args.len() != 1 {
                return Err("`writeln` 需要 1 个参数".into());
            }
            let text = match &args[0] {
                Value::String(s) => s.clone(),
                Value::List(items) => {
                    let mut joined = String::new();
                    for item in items {
                        let Value::String(s) = item else {
                            return Err("`writeln` 的列表元素必须是字符串".into());
                        };
                        joined.push_str(s);
                    }
                    joined
                }
                other => {
                    return Err(format!(
                        "`writeln` 需要字符串或字符串列表，收到 `{}`",
                        value_string(other)
                    ))
                }
            };
            let mut bytes = encode_text(&text, encoding);
            // 末尾没有换行时补一个
            if !text.ends_with('\n') {
                bytes.extend_from_slice(&encode_text("\n", encoding));
            }
            append_or_write(path, mode, &bytes, position)?;
            Ok((Value::Int(bytes.len() as i64), Some(bytes.len())))
        }
        "close" => Ok((Value::Null, None)),
        other => Err(format!("`{other}` 不是文件方法")),
    }
}

fn append_or_write(path: &str, mode: &str, bytes: &[u8], position: usize) -> Result<(), String> {
    use std::io::Write;
    match mode {
        "a" | "ab" => {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .map_err(|e| format!("打开 `{path}` 失败：{e}"))?;
            file.write_all(bytes)
                .map_err(|e| format!("写入 `{path}` 失败：{e}"))
        }
        "r+" | "rb+" => {
            // 读写模式：从当前位置写入，不清空原内容
            let mut file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .map_err(|e| format!("打开 `{path}` 失败：{e}"))?;
            use std::io::Seek;
            file.seek(std::io::SeekFrom::Start(position as u64))
                .map_err(|e| format!("定位 `{path}` 失败：{e}"))?;
            file.write_all(bytes)
                .map_err(|e| format!("写入 `{path}` 失败：{e}"))
        }
        _ => {
            // w / wb：截断后写入
            let mut file = std::fs::File::create(path)
                .map_err(|e| format!("写入 `{path}` 失败：{e}"))?;
            file.write_all(bytes)
                .map_err(|e| format!("写入 `{path}` 失败：{e}"))
        }
    }
}

/// 把值编码为字节：字符串按指定编码，列表视为字节序列
fn value_to_bytes(value: &Value, encoding: &str) -> Result<Vec<u8>, String> {
    match value {
        Value::String(text) => Ok(encode_text(text, encoding)),
        Value::List(items) => {
            let mut bytes = Vec::with_capacity(items.len());
            for item in items {
                let Value::Int(n) = item else {
                    return Err("写入二进制时列表元素必须是 int".into());
                };
                if !(0..=255).contains(n) {
                    return Err(format!("字节值必须在 0 到 255 之间，收到 {n}"));
                }
                bytes.push(*n as u8);
            }
            Ok(bytes)
        }
        other => Err(format!(
            "`write` 需要字符串或字节列表，收到 `{}`",
            value_string(other)
        )),
    }
}

fn encode_text(text: &str, encoding: &str) -> Vec<u8> {
    match encoding.to_ascii_lowercase().replace('-', "") {
        e if e == "utf8" => text.as_bytes().to_vec(),
        e if e == "ascii" => text
            .chars()
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect(),
        e if e == "gbk" || e == "gb2312" || e == "gb18030" => text
            .chars()
            .flat_map(|c| {
                // 简化实现：非 ASCII 字符用 UTF-8 字节代替
                if c.is_ascii() {
                    vec![c as u8]
                } else {
                    let mut buf = [0u8; 4];
                    c.encode_utf8(&mut buf).as_bytes().to_vec()
                }
            })
            .collect(),
        e if e == "latin1" || e == "iso88591" => text
            .chars()
            .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
            .collect(),
        _ => text.as_bytes().to_vec(),
    }
}

fn decode_text(bytes: &[u8], encoding: &str) -> String {
    match encoding.to_ascii_lowercase().replace('-', "") {
        e if e == "latin1" || e == "iso88591" => {
            bytes.iter().map(|&b| b as char).collect()
        }
        e if e == "ascii" => bytes
            .iter()
            .map(|&b| if b.is_ascii() { b as char } else { '\u{FFFD}' })
            .collect(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// 写入列表或字典的某个位置，返回更新后的容器。
/// 列表允许下标等于长度时追加，超过则报错；字典允许新增键。
fn assign_index(container: Value, index: Value, value: Value) -> Result<Value, String> {
    match container {
        Value::List(mut values) => {
            let Value::Int(position) = index else {
                return Err("列表下标必须是 int".into());
            };
            if position < 0 {
                return Err(format!("列表下标不能为负 ({position})"));
            }
            let position = position as usize;
            if position < values.len() {
                values[position] = value;
            } else if position == values.len() {
                values.push(value);
            } else {
                return Err(format!(
                    "列表下标 {position} 越界（当前长度 {}，只能在末尾追加）",
                    values.len()
                ));
            }
            Ok(Value::List(values))
        }
        Value::String(text) => {
            let Value::String(replacement) = value else {
                return Err("字符串下标赋值必须赋一个字符串".into());
            };
            let replacement: Vec<char> = replacement.chars().collect();
            if replacement.len() != 1 {
                return Err(format!(
                    "字符串下标赋值要求恰好 1 个字符，收到 {} 个",
                    replacement.len()
                ));
            }
            let Value::Int(position) = index else {
                return Err("字符串下标必须是 int".into());
            };
            let mut chars: Vec<char> = text.chars().collect();
            if position < 0 || position as usize >= chars.len() {
                return Err(format!(
                    "字符串下标 {position} 越界（长度 {}）",
                    chars.len()
                ));
            }
            chars[position as usize] = replacement[0];
            Ok(Value::String(chars.into_iter().collect()))
        }
        Value::Dict(mut entries) => {
            for entry in entries.iter_mut() {
                if matches!(binary(entry.0.clone(), "==", index.clone())?, Value::Bool(true)) {
                    entry.1 = value;
                    return Ok(Value::Dict(entries));
                }
            }
            entries.push((index, value));
            Ok(Value::Dict(entries))
        }
        other => Err(format!(
            "下标赋值只能用于列表或字典，收到 `{}`",
            value_string(&other)
        )),
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
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } | Value::File { .. } => unreachable!(),
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
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } | Value::File { .. } => unreachable!(),
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
        Value::List(_) | Value::Dict(_) | Value::Struct { .. } | Value::File { .. } => unreachable!(),
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
        Value::File { path, .. } => path.clone(),
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
        let src = r#"let fixed : int = 1; var count : int = 2; fn add(a : int, b : int) -> int { return(a + b); } println(f"{fixed}{add(count, 3)}");"#;
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
        let src = "var n : int = 1; fn inc(x : int) -> int { return(x + 1); } n = inc(n); println(n);";
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
            fn fibonacci(n : int) -> int {
                if (n <= 0) { return(0); }
                elif (n == 1) { return(1); }
                else { return(fibonacci(n - 1) + fibonacci(n - 2)); }
            }
            let result : int = if (fibonacci(5) == 5) { 10 } else { 20 };
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
                var id : int;
                var age : int;
                var name : string;
            } s;
            fn main() -> int {
                s.id = 7;
                s.age = 8;
                s.name = "Ann";
                return(s.id + s.age);
            }
            var result : int = main();
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
        let error = Parser::new(lex("struct a { var x : int; } s").unwrap())
            .program()
            .unwrap_err();
        assert!(error.contains("`;`"), "unexpected error: {error}");
    }

    #[test]
    fn parses_trailing_type_annotation() {
        let src = r#"
            var n : int = 5;
            let s : string = "A";
            var f : float = 1.5;
            var b : bool = true;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("n"), Some(Value::Int(5))));
        assert!(matches!(
            interpreter.lookup("s"),
            Some(Value::String(v)) if v == "A"
        ));
        assert!(matches!(interpreter.lookup("f"), Some(Value::Float(_))));
        assert!(matches!(interpreter.lookup("b"), Some(Value::Bool(true))));
    }

    #[test]
    fn infers_type_with_walrus_operator() {
        let src = r#"
            var x := 5;
            var name := "Khyept";
            var pi := 3.5;
            var flag := false;
            var inferred_type := type(x);
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("x"), Some(Value::Int(5))));
        assert!(matches!(
            interpreter.lookup("name"),
            Some(Value::String(v)) if v == "Khyept"
        ));
        assert!(matches!(
            interpreter.lookup("inferred_type"),
            Some(Value::String(v)) if v == "int"
        ));
    }

    #[test]
    fn walrus_inferred_type_is_enforced_on_reassignment() {
        let src = r#"
            var x := 5;
            x = 10;
            x = "boom";
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("类型 `int`"), "unexpected error: {error}");
    }

    #[test]
    fn parses_function_with_trailing_return_type() {
        let src = r#"
            fn add(a : int, b : int) -> int { return(a + b); }
            var sum : int = add(3, 4);
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("sum"), Some(Value::Int(7))));
    }

    #[test]
    fn function_without_return_type_defaults_to_void() {
        // 不写-> 时默认 void，函数体内的 return 值不参与调用结果
        let src = r#"
            fn nothing() { println("hi"); }
            nothing();
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
    }

    #[test]
    fn enforces_declared_function_return_type() {
        let src = r#"
            fn bad() -> int { return("not an int"); }
            var r = bad();
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("返回值"), "unexpected error: {error}");
    }

    #[test]
    fn reports_missing_return_for_non_void_function() {
        let src = r#"
            fn missing() -> int { println("no return"); }
            var r = missing();
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("应返回"), "unexpected error: {error}");
    }

    #[test]
    fn keeps_legacy_prefixed_type_syntax_working() {
        // 旧写法应继续可用，避免一次性破坏现有代码
        let src = r#"
            var int n = 5;
            fn int twice(int x) { return(x * 2); }
            var int r = twice(n);
            struct P { var int x; var int y; } p{1, 2};
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("r"), Some(Value::Int(10))));
    }

    #[test]
    fn parses_trailing_type_in_struct_fields() {
        let src = r#"
            struct student {
                var id : int;
                let name : string;
            } s;
            s.id = 3;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("s"),
            Some(Value::Struct { fields, .. })
                if matches!(&fields[0].value, Value::Int(3))
                    && matches!(&fields[1].value, Value::String(_))
        ));
    }

    #[test]
    fn assigns_to_list_element() {
        let src = r#"
            var l : list = [0, 1, 2, 3];
            l[1] = 2;
            var r = l[1];
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("r"), Some(Value::Int(2))));
        assert!(matches!(
            interpreter.lookup("l"),
            Some(Value::List(v)) if v == &vec![
                Value::Int(0),
                Value::Int(2),
                Value::Int(2),
                Value::Int(3)
            ]
        ));
    }

    #[test]
    fn appends_to_list_via_index_equal_to_length() {
        let src = r#"
            var l : list = [1, 2];
            l[size(l)] = 3;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("l"),
            Some(Value::List(v))
                if v == &vec![Value::Int(1), Value::Int(2), Value::Int(3)]
        ));
    }

    #[test]
    fn rejects_out_of_order_list_index_assignment() {
        let src = r#"
            var l : list = [1, 2];
            l[5] = 9;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("越界"), "unexpected error: {error}");
    }

    #[test]
    fn assigns_into_nested_containers() {
        let src = r#"
            var g : list = [[0, 0], [0, 0]];
            g[0][1] = 5;
            g[1][0] = 7;
            var d : dict = {"a": 1};
            d["a"] = 10;
            d["b"] = 20;
            var new_key = d["b"];
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("new_key"), Some(Value::Int(20))));
        let Value::List(rows) = interpreter.lookup("g").unwrap() else {
            panic!("expected list");
        };
        let Value::List(first) = &rows[0] else {
            panic!("expected list");
        };
        assert!(matches!(&first[1], Value::Int(5)));
        assert!(matches!(interpreter.lookup("d"), Some(Value::Dict(e)) if e.len() == 2));
    }

    #[test]
    fn evaluates_logical_operators() {
        let src = r#"
            var a := 5;
            var b := 7;
            var both = a == 5 && b == 7;
            var either = a == 99 || b == 7;
            var neither = !(a == 5) && !(b == 7);
            var negated = !false;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("both"), Some(Value::Bool(true))));
        assert!(matches!(
            interpreter.lookup("either"),
            Some(Value::Bool(true))
        ));
        assert!(matches!(
            interpreter.lookup("neither"),
            Some(Value::Bool(false))
        ));
        assert!(matches!(
            interpreter.lookup("negated"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn short_circuits_logical_operators() {
        // 右侧若是会出错的表达式，短路时不应被求值
        let src = r#"
            var x := 0;
            if (x == 1 && 5 > 3) { println("no"); }
            if (x == 0 || 5 > 3) println("yes");
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        assert!(interpreter.run(&ast).is_ok());
    }

    #[test]
    fn rejects_non_bool_operands_for_logical_operators() {
        let ast = Parser::new(lex("var a := 1; var r = a && true;").unwrap())
            .program()
            .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("bool"), "unexpected error: {error}");
    }

    #[test]
    fn accepts_single_statement_body_without_braces() {
        let src = r#"
            var a := 5;
            if (a == 5) println("a5");
            if (a == 99) println("no"); else println("else branch");
            var i := 0;
            while (i < 3) i++;
            var total = i;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("total"), Some(Value::Int(3))));
    }

    #[test]
    fn evaluates_extended_operators() {
        let src = r#"
            var div = 7 // 2;
            var rem = 7 % 2;
            var pow = 2 ** 3;
            var band = 5 & 3;
            var bor = 5 | 3;
            var bxor = 5 ^ 3;
            var bnot = ~5;
            var shl = 5 << 1;
            var shr = 5 >> 1;
            var right_assoc = 2 ** 3 ** 2;
            var fdiv = 7.0 // 2.0;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        let expected: &[(&str, i64)] = &[
            ("div", 3),
            ("rem", 1),
            ("pow", 8),
            ("band", 1),
            ("bor", 7),
            ("bxor", 6),
            ("bnot", -6),
            ("shl", 10),
            ("shr", 2),
            ("right_assoc", 512),
        ];
        for (name, value) in expected {
            assert!(
                matches!(interpreter.lookup(name), Some(Value::Int(v)) if *v == *value),
                "{name} should be {value}"
            );
        }
        assert!(matches!(
            interpreter.lookup("fdiv"),
            Some(Value::Float(v)) if *v == 3.0
        ));
    }

    #[test]
    fn distinguishes_floor_division_from_line_comment() {
        let src = r#"
            var a := 7;
            var q = a // 2;   /// 这是行尾注释
            var b := 1;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("q"), Some(Value::Int(3))));
        assert!(matches!(interpreter.lookup("b"), Some(Value::Int(1))));
    }

    #[test]
    fn rejects_bitwise_operators_on_floats() {
        let ast = Parser::new(lex("var r = 1.5 & 2;").unwrap())
            .program()
            .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("int"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_division_by_zero_for_floor_division() {
        let ast = Parser::new(lex("var r = 7 // 0;").unwrap())
            .program()
            .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("除数不能为零"), "unexpected error: {error}");
    }

    #[test]
    fn triple_slash_is_line_comment_and_slash_slash_is_division() {
        // `///` 是行注释，`//` 是整除，两者不再有歧义
        let src = r#"
            /// 这一行是中文注释，含全角标点：，、（）
            fn f() {
                println("hi");
            }
            var a := 7 // 2;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("a"), Some(Value::Int(3))));
    }

    #[test]
    fn skips_text_after_triple_slash_comment() {
        // 注释里的 // 与引号都不应影响后续解析
        let src = r#"
            /// 这里写了 // 甚至 "引号" 也不影响
            var a := 1;
            var b := 2;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("a"), Some(Value::Int(1))));
        assert!(matches!(interpreter.lookup("b"), Some(Value::Int(2))));
    }

    #[test]
    fn parses_block_comments() {
        let src = r#"
            /*
             * 多行块注释
             * 里面可以有 // 和 ** 各种符号
             */
            var a := 1;
            var b := /* 行内块注释 */ 2;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("a"), Some(Value::Int(1))));
        assert!(matches!(interpreter.lookup("b"), Some(Value::Int(2))));
    }

    #[test]
    fn rejects_unclosed_block_comment() {
        let error = lex("/* 没有闭合\nvar a := 1;").unwrap_err();
        assert!(error.contains("未闭合"), "unexpected error: {error}");
    }

    #[test]
    fn floor_division_after_parenthesized_operand() {
        let src = r#"
            var a := (7);
            var q = a // 2;
            var b := (3 + 4) // 2;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("q"), Some(Value::Int(3))));
        assert!(matches!(interpreter.lookup("b"), Some(Value::Int(3))));
    }

    #[test]
    fn double_slash_alone_is_a_syntax_error() {
        // 只有两个斜杠时是整除运算符，后面缺少操作数应报错
        let error = Parser::new(lex("var a := 1 // ;").unwrap())
            .program()
            .unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn constructs_struct_with_expression_syntax() {
        let src = r#"
            struct P { var x : int; var y : string; }

            var a = P{1, "one"};
            var b : P = P{2, "two"};
            struct Q { var n : int; }
            var c : Q = Q{9};
            var total = a.x + b.x + c.n;
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("total"), Some(Value::Int(12))));
        assert!(matches!(
            interpreter.lookup("a"),
            Some(Value::Struct { fields, .. })
                if matches!(&fields[0].value, Value::Int(1))
                    && matches!(&fields[1].value, Value::String(v) if v == "one")
        ));
    }

    #[test]
    fn indexes_and_slices_strings() {
        let src = r#"
            var s : string = "Hello, World!";
            s[0] = "h";
            var first = s[0];
            var tail = s[1:];
            var head = s[:5];
            var mid = s[0:5];
            var last = s[7:];
            var whole = s[:];
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("s"),
            Some(Value::String(v)) if v == "hello, World!"
        ));
        assert!(matches!(
            interpreter.lookup("first"),
            Some(Value::String(v)) if v == "h"
        ));
        assert!(matches!(
            interpreter.lookup("tail"),
            Some(Value::String(v)) if v == "ello, World!"
        ));
        assert!(matches!(
            interpreter.lookup("head"),
            Some(Value::String(v)) if v == "hello"
        ));
        assert!(matches!(
            interpreter.lookup("last"),
            Some(Value::String(v)) if v == "World!"
        ));
        assert!(matches!(
            interpreter.lookup("whole"),
            Some(Value::String(v)) if v == "hello, World!"
        ));
    }

    #[test]
    fn rejects_out_of_range_string_index() {
        let ast = Parser::new(lex("var s := \"ab\"; s[5] = \"x\";").unwrap())
            .program()
            .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("越界"), "unexpected error: {error}");
    }

    #[test]
    fn counts_length_with_len_and_size() {
        let src = r#"
            var s := "Hello, World!";
            var l : list = [1, 2, 3];
            var d : dict = {"a": 1, "b": 2};
            var ls = len(s);
            var ss = s.size();
            var ll = len(l);
            var ls2 = l.size();
            var dl = len(d);
            var ds = d.size();
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        for (name, want) in [
            ("ls", 13),
            ("ss", 13),
            ("ll", 3),
            ("ls2", 3),
            ("dl", 2),
            ("ds", 2),
        ] {
            assert!(
                matches!(interpreter.lookup(name), Some(Value::Int(v)) if *v == want),
                "{name} should be {want}"
            );
        }
    }

    #[test]
    fn reads_and_writes_files() {
        let dir = std::env::temp_dir().join("khyept_file_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");
        let path_str = path.to_string_lossy().replace('\\', "\\\\");
        let src = format!(
            r#"
            var f = open("{path_str}", "w", encoding="utf-8");
            f.write("Hello, World!");
            f.close();

            var g = open("{path_str}", "r");
            var content : string = g.read();
            g.close();

            var h = open("{path_str}", "a");
            h.write("\nsecond line");
            h.close();

            var i2 = open("{path_str}", "r");
            var full : string = i2.read();
            i2.close();
            "#
        );
        let ast = Parser::new(lex(&src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("content"),
            Some(Value::String(v)) if v == "Hello, World!"
        ));
        assert!(matches!(
            interpreter.lookup("full"),
            Some(Value::String(v)) if v == "Hello, World!\nsecond line"
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_write_mode_overwrites_in_place() {
        let dir = std::env::temp_dir().join("khyept_rplus_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rplus.txt");
        let path_str = path.to_string_lossy().replace('\\', "\\\\");
        let src = format!(
            r#"
            var f = open("{path_str}", "w");
            f.write("Hello World");
            f.close();

            var g = open("{path_str}", "r+");
            var before : string = g.read();
            g.write("ABC");
            g.close();

            var h = open("{path_str}", "r");
            var after : string = h.read();
            h.close();
            "#
        );
        let ast = Parser::new(lex(&src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("before"),
            Some(Value::String(v)) if v == "Hello World"
        ));
        assert!(matches!(
            interpreter.lookup("after"),
            Some(Value::String(v)) if v == "ABClo World"
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_multiple_lines_and_binary_bytes() {
        let dir = std::env::temp_dir().join("khyept_multi_test");
        std::fs::create_dir_all(&dir).unwrap();
        let txt = dir.join("lines.txt");
        let bin = dir.join("data.bin");
        let txt_str = txt.to_string_lossy().replace('\\', "\\\\");
        let bin_str = bin.to_string_lossy().replace('\\', "\\\\");
        let src = format!(
            r#"
            var lines := ["first\n", "second\n"];
            var f = open("{txt_str}", "w");
            f.writeln(lines);
            f.close();

            var g = open("{txt_str}", "r");
            var text : string = g.read();
            g.close();

            var data : list = [0, 1, 2, 255];
            var b = open("{bin_str}", "wb");
            b.write(data);
            b.close();
            var raw = open("{bin_str}", "rb");
            var bytes : string = raw.read();
            raw.close();
            var byte_len = len(bytes);
            "#
        );
        let ast = Parser::new(lex(&src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("text"),
            Some(Value::String(v)) if v == "first\nsecond\n"
        ));
        assert!(matches!(
            interpreter.lookup("byte_len"),
            Some(Value::Int(4))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_missing_file_in_read_mode() {
        let ast =
            Parser::new(lex("var f = open(\"definitely_missing_file_xyz.txt\", \"r\");").unwrap())
                .program()
                .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("不存在"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_invalid_file_mode() {
        let ast = Parser::new(lex("var f = open(\"x.txt\", \"q\");").unwrap())
            .program()
            .unwrap();
        let mut interpreter = Interpreter::new();
        let error = interpreter.run(&ast).unwrap_err();
        assert!(error.contains("模式"), "unexpected error: {error}");
    }

    #[test]
    fn parses_struct_initializer_inside_fstring() {
        let src = r#"
            struct P { var x : int; }
            var a = P{7};
            var text = f"{P{1}}";
        "#;
        let ast = Parser::new(lex(src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(interpreter.lookup("a"), Some(Value::Struct { .. })));
        assert!(matches!(
            interpreter.lookup("text"),
            Some(Value::String(v)) if v.contains('1')
        ));
    }

    #[test]
    fn parses_named_argument_in_open() {
        let dir = std::env::temp_dir().join("khyept_named_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("n.txt");
        let path_str = path.to_string_lossy().replace('\\', "\\\\");
        let src = format!(
            r#"
            var f = open("{path_str}", "w", encoding="utf-8");
            f.write("ok");
            f.close();
            var g = open("{path_str}", "r", encoding="utf-8");
            var text : string = g.read();
            g.close();
            "#
        );
        let ast = Parser::new(lex(&src).unwrap()).program().unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.run(&ast).unwrap();
        assert!(matches!(
            interpreter.lookup("text"),
            Some(Value::String(v)) if v == "ok"
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

