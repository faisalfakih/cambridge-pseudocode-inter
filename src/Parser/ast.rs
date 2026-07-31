use crate::{Inter::cps::{Type, Value}, Lexer::lexer::TokenType};

#[derive(Debug, Clone, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    Append,
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub precedence: Precedence,
    pub position: Position,
    pub associativity: Associativity,
}

pub type Precedence = u16;

#[derive(Debug, Clone)]
pub enum Position {
    Prefix,
    Infix,
    Postfix,
}

#[derive(Debug, Clone)]
pub enum Associativity {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub left: Box<Ast>,
    pub operator: TokenType,
    pub right: Box<Ast>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Identifier(String),
    Expression(Expr),
    Stmt(Stmt)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary(BinaryExpr),
    Literal(Value),
    Call { name: String, arguments: Vec<Expr> },
    ArrayAccess { name: String, index: Box<Expr>, col: Option<Box<Expr>> }, // col for 2D arrays, the index exists for both 1D and 2D arrays but would be the row in a 2d array     
    EOF { filename: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// How the value is passed (by value or by reference)
pub enum PassingValue {
    ByVal,
    ByRef,
}




#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    If { condition: Box<Expr>, then_branch: BlockStmt, else_branch: Option<BlockStmt> },
    Case { identifier: Box<Expr>, cases: Vec<(CaseCondition, BlockStmt)>, otherwise: Option<BlockStmt> },
    While { condition: Box<Expr>, body: BlockStmt },
    Repeat { body: BlockStmt, until: Box<Expr> },
    For { identifier: String, start: Box<Expr>, end: Box<Expr>, body: BlockStmt, step: Box<Expr> },
    Assignment { identifier: String, array_index: Option<(Box<Expr>, Option<Box<Expr>>)>, value: Box<Ast> },
    Constant { identifier: String, value: Value },
    Decleration { identifier: String, type_: Type},
    Input { identifier: Box<Expr> }, 
    Output { target: Expr },
    Block(BlockStmt),
    Procedure {
        name: String,
        parameters: Vec<(String, Type, PassingValue)>,
        body: BlockStmt,
    },
    Function {
        name: String,
        parameters: Vec<(String, Type, PassingValue)>,
        return_type: Type,
        body: BlockStmt,
    },
    Return { value: Box<Expr> },
    Call { name: String, arguments: Vec<Expr> }, 
    OpenFile { filename: Box<Expr>, mode: FileMode },
    CloseFile { filename: Box<Expr> },
    ReadFile { filename: Box<Expr>, target: Box<Expr> },
    WriteFile { filename: Box<Expr>, value: Box<Expr> },
    TypeDef { type_definition: TypeDefinition },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    Enumerated { name: String, values: Vec<String> }, // TYPE Season = (Spring, Summer, Autumn, Winter)
    Pointer { name: String, points_to: Box<Type> }, // TYPE TypePointer = ^Type
    // TYPE StudentRecord
    //    DECLARE LastName : STRING
    //    ...
    // ENDTYPE
    Record { name: String, fields: Vec<(String, Box<Type>)> }, 
    // TYPE LetterSet = SET OF CHAR
    // DEFINE Vowels ('A','E','I','O','U') : LetterSet
    Set { name: String, base_type: Box<Type> }, 
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseArm {
    pub condition: CaseCondition,
    pub statements: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseCondition {
    Single(Expr),
    Range(Expr, Expr), // value1 TO value2
}



#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
}

impl BlockStmt {
    pub fn new(statements: Vec<Stmt>) -> Self {
        BlockStmt { statements }
    }
}




impl Expr {
    fn to_prefix(&self) -> String {
        match self {
            Expr::Binary(binary) => {
                let op_str = match binary.operator {
                    TokenType::Plus => "+",
                    TokenType::Minus => "-",
                    TokenType::Asterisk => "*",
                    TokenType::ForwardSlash => "/",
                    TokenType::Caret => "^",
                    TokenType::And => "AND",
                    TokenType::Or => "OR",
                    TokenType::Div => "DIV",
                    TokenType::Mod => "MOD",
                    TokenType::Equal => "=",
                    TokenType::NotEqual => "<>",
                    TokenType::LessThan => "<",
                    TokenType::LessEqual => "<=",
                    TokenType::GreaterThan => ">",
                    TokenType::GreaterEqual => ">=",
                    _ => "?",
                };
                format!(
                    "({} {} {})",
                    op_str,
                    binary.left.to_prefix(),
                    binary.right.to_prefix()
                )
            }
            Expr::Literal(value) => {
                match value {
                    Value::Integer(n) => n.to_string(),
                    Value::Real(f) => f.to_string(),
                    Value::String(s) => format!("\"{}\"", s),
                    Value::Boolean(b) => b.to_string(),
                    Value::Char(c) => format!("'{}'", c),
                    _ => "?".to_string(),
                }
            }
            Expr::Call { name, arguments } => {
                let args: Vec<String> = arguments.iter().map(|arg| arg.to_prefix()).collect();
                format!("CALL {}({})", name, args.join(", "))
            },
            Expr::ArrayAccess { name, index, col } => {
                if let Some(col_expr) = col {
                    format!("{}[{}, {}]", name, index.to_prefix(), col_expr.to_prefix())
                } else {
                    format!("{}[{}]", name, index.to_prefix())
                }
            }
            Expr::EOF { filename } => format!("EOF({})", filename),
        }
    }
}

impl Stmt {
    fn to_prefix(&self, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        match self {
            Stmt::If { condition, then_branch, else_branch } => {
                let mut result = format!("{}IF {}\n", indent_str, condition.to_prefix());
                result.push_str(&format!("{}THEN\n", indent_str));
                for stmt in &then_branch.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                if let Some(else_block) = else_branch {
                    result.push_str(&format!("{}ELSE\n", indent_str));
                    for stmt in &else_block.statements {
                        result.push_str(&stmt.to_prefix(indent + 1));
                        result.push('\n');
                    }
                }
                result.push_str(&format!("{}ENDIF", indent_str));
                result
            }
            Stmt::Case { identifier, cases, otherwise } => {
                let mut result = format!("{}CASE {}\n", indent_str, identifier.to_prefix());
                for (condition, block) in cases {
                    match condition {
                        CaseCondition::Single(expr) => {
                            result.push_str(&format!("{}  WHEN {} THEN\n", indent_str, expr.to_prefix()));
                        }
                        CaseCondition::Range(start, end) => {
                            result.push_str(&format!("{}  WHEN {} TO {} THEN\n", indent_str, start.to_prefix(), end.to_prefix()));
                        }
                    }
                    for stmt in &block.statements {
                        result.push_str(&stmt.to_prefix(indent + 2));
                        result.push('\n');
                    }
                }
                if let Some(otherwise_block) = otherwise {
                    result.push_str(&format!("{}  OTHERWISE\n", indent_str));
                    for stmt in &otherwise_block.statements {
                        result.push_str(&stmt.to_prefix(indent + 2));
                        result.push('\n');
                    }
                }
                result.push_str(&format!("{}ENDCASE", indent_str));
                result
            }
            Stmt::Repeat { body, until } => {
                let mut result = format!("{}REPEAT\n", indent_str);
                for stmt in &body.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}UNTIL {}", indent_str, until.to_prefix()));
                result
            }
            Stmt::While { condition, body } => {
                let mut result = format!("{}WHILE {}\n", indent_str, condition.to_prefix());
                for stmt in &body.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}ENDWHILE", indent_str));
                result
            }
            Stmt::For { identifier, start, end, body, step } => {
                let mut result = format!("{}FOR {} = {} TO {} STEP {}\n", indent_str, identifier, start.to_prefix(), end.to_prefix(), step.to_prefix());
                for stmt in &body.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}ENDFOR", indent_str));
                result
            }
            Stmt::Assignment { identifier, array_index, value } => {
                if let Some((index_expr, col_expr)) = array_index {
                    if let Some(col) = col_expr {
                        format!("{}{}[{}, {}] := {}", indent_str, identifier, index_expr.to_prefix(), col.to_prefix(), value.to_prefix())
                    } else {
                        format!("{}{}[{}] := {}", indent_str, identifier, index_expr.to_prefix(), value.to_prefix())
                    }
                } else {
                    format!("{}{} := {}", indent_str, identifier, value.to_prefix())
                }
            }
            Stmt::Decleration { identifier, type_ } => {
                format!("{}DECLARE {} : {:?}", indent_str, identifier, type_)
            }
            Stmt::Constant { identifier, value } => {
                format!("{}CONST {} = {:?}", indent_str, identifier, value)
            }
            Stmt::Input { identifier } => {
                format!("{}INPUT {}", indent_str, identifier.to_prefix())
            }
            Stmt::Output { target } => {
                format!("{}OUTPUT {}", indent_str, target.to_prefix())
            }
            Stmt::Block(block) => {
                let mut result = format!("{}BEGIN\n", indent_str);
                for stmt in &block.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}END", indent_str));
                result
            }
            Stmt::Procedure { name, parameters, body } => {
                let mut result = format!("{}PROCEDURE {}(", indent_str, name);
                let params: Vec<String> = parameters.iter().map(|(n, t, p)| format!("{:?} {}: {:?}", p, n, t)).collect();
                result.push_str(&params.join(", "));
                result.push_str(")\n");
                for stmt in &body.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}ENDPROCEDURE", indent_str));
                result
            }
            Stmt::Function { name, parameters, return_type, body } => {
                let mut result = format!("{}FUNCTION {}(", indent_str, name);
                let params: Vec<String> = parameters.iter().map(|(p, t, b)| format!("{:?} {}: {:?}", b, p, t)).collect();
                result.push_str(&params.join(", "));
                result.push_str(&format!(") : {:?}\n", return_type));
                for stmt in &body.statements {
                    result.push_str(&stmt.to_prefix(indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}ENDFUNCTION", indent_str));
                result
            }
            Stmt::Call { name, arguments } => {
                let args: Vec<String> = arguments.iter().map(|arg| arg.to_prefix()).collect();
                format!("{}CALL {}({})", indent_str, name, args.join(", "))
            }
            Stmt::Return { value } => {
                format!("{}RETURN {:?}", indent_str, *value)
            }
            Stmt::OpenFile { filename, mode } => {
                let mode_str = match mode {
                    FileMode::Read => "READ",
                    FileMode::Write => "WRITE",
                    FileMode::Append => "APPEND",
                };
                format!("{}OPENFILE {} MODE {}", indent_str, filename.to_prefix(), mode_str)
            }
            Stmt::CloseFile { filename } => {
                format!("{}CLOSEFILE {}", indent_str, filename.to_prefix())
            }
            Stmt::ReadFile { filename, target } => {
                format!("{}READFILE {} INTO {}", indent_str, filename.to_prefix(), target.to_prefix())
            }
            Stmt::WriteFile { filename, value } => {
                format!("{}WRITEFILE {} FROM {}", indent_str, filename.to_prefix(), value.to_prefix())
            }
            Stmt::TypeDef { type_definition } => {
                match type_definition {
                    TypeDefinition::Enumerated { name, values } => {
                        let vals = values.join(", ");
                        format!("{}TYPE {} = ({})", indent_str, name, vals)
                    }
                    TypeDefinition::Pointer { name, points_to } => {
                        format!("{}TYPE {} = ^{:?}", indent_str, name, points_to)
                    }
                    TypeDefinition::Record { name, fields } => {
                        let mut result = format!("{}TYPE {} = RECORD\n", indent_str, name);
                        for (field_name, field_type) in fields {
                            result.push_str(&format!("{}  {}: {:?};\n", indent_str, field_name, field_type));
                        }
                        result.push_str(&format!("{}ENDRECORD", indent_str));
                        result
                    }
                    TypeDefinition::Set { name, base_type } => {
                        format!("{}TYPE {} = SET OF {:?}", indent_str, name, base_type)
                    }
                }
            }
        }
    }
}

impl Ast {
    fn to_prefix(&self) -> String {
        match self {
            Ast::Identifier(name) => name.clone(),
            Ast::Stmt(stmt) => stmt.to_prefix(0),
            Ast::Expression(expr) => expr.to_prefix(),
        }
    }
    
    pub fn print_prefix(&self) {
        println!("{}", self.to_prefix());
    }
}
