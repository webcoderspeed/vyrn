/// Abstract Syntax Tree nodes for Vryn
/// Every Vryn program is parsed into these structures

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

// === Statements ===

#[derive(Debug, Clone)]
pub enum Statement {
    /// let x = 5 or let mut x = 5
    Let {
        name: String,
        mutable: bool,
        type_ann: Option<String>,  // optional type annotation
        value: Expression,
    },
    /// fn name(params) -> return_type { body }
    Function {
        name: String,
        params: Vec<Param>,
        return_type: Option<String>,
        body: Vec<Statement>,
    },
    /// struct Name { fields }
    Struct {
        name: String,
        fields: Vec<Field>,
    },
    /// enum Name { variants }
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    /// trait Name { methods }
    Trait {
        name: String,
        methods: Vec<TraitMethod>,
    },
    /// impl TraitName for TypeName { methods } OR impl TypeName { methods }
    Impl {
        trait_name: Option<String>,  // None for impl without trait
        type_name: String,
        methods: Vec<ImplMethod>,
    },
    /// import path or use path
    Import {
        path: String,
        alias: Option<String>,
    },
    /// expression as statement (function call, assignment, etc.)
    Expression(Expression),
    /// return value
    Return(Option<Expression>),
    /// if condition { body } else { body }
    If {
        condition: Box<Expression>,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    /// if let pattern = expr { body } else { body }
    IfLet {
        pattern: Pattern,
        expr: Box<Expression>,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    /// while condition { body }
    While {
        condition: Box<Expression>,
        body: Vec<Statement>,
    },
    /// while let pattern = expr { body }
    WhileLet {
        pattern: Pattern,
        expr: Box<Expression>,
        body: Vec<Statement>,
    },
    /// for var in iterable { body }
    For {
        var: String,
        iterable: Box<Expression>,
        body: Vec<Statement>,
    },
    /// break
    Break,
    /// continue
    Continue,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<String>, // type names
}

/// A method definition in a trait
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
}

/// A method implementation in an impl block
#[derive(Debug, Clone)]
pub struct ImplMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
}

// === Expressions ===

#[derive(Debug, Clone)]
pub enum Expression {
    /// Integer literal: 42
    IntLiteral(i64),
    /// Float literal: 3.14
    FloatLiteral(f64),
    /// String literal: "hello"
    StringLiteral(String),
    /// Boolean: true, false
    BoolLiteral(bool),
    /// Variable reference: x, name, foo
    Identifier(String),
    /// Binary operation: a + b, x == y
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    /// Unary operation: -x, !flag
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },
    /// Function call: foo(a, b)
    Call {
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    /// Member access: obj.field
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
    /// Index access: arr[0]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    /// Assignment: x = 5
    Assign {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    /// Array literal: [1, 2, 3]
    Array(Vec<Expression>),
    /// Pipe: a |> b |> c
    Pipe {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Range: 0..10 or 0..=10
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    /// Match expression
    Match {
        value: Box<Expression>,
        arms: Vec<MatchArm>,
    },
    /// Block expression: { stmts; expr }
    Block(Vec<Statement>),
    /// Struct instantiation: Point { x: 1, y: 2 }
    StructInit {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    /// Lambda: |x, y| x + y
    Lambda {
        params: Vec<String>,
        body: Box<Expression>,
    },
    /// Try-catch: try { ... } catch e { ... }
    TryCatch {
        try_body: Vec<Statement>,
        catch_var: String,
        catch_body: Vec<Statement>,
    },
    /// Question mark operator: expr?
    QuestionMark {
        expr: Box<Expression>,
    },
    /// Method call: obj.method(args)
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    /// Self reference
    Self_,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expression,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// Matches a literal value
    Literal(Expression),
    /// Matches and binds to a name
    Identifier(String),
    /// Wildcard: _
    Wildcard,
    /// Enum variant: Some(x)
    EnumVariant {
        name: String,
        fields: Vec<Pattern>,
    },
    /// Tuple pattern: (a, b, c)
    Tuple(Vec<Pattern>),
    /// Struct pattern: Point { x, y }
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    /// Range pattern: 0..10 or 0..=10
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    /// Or-pattern: A | B | C
    Or(Vec<Pattern>),
    /// Guard clause: pattern if condition
    Guard {
        pattern: Box<Pattern>,
        condition: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,        // +
    Sub,        // -
    Mul,        // *
    Div,        // /
    Mod,        // %
    Eq,         // ==
    NotEq,      // !=
    Less,       // <
    Greater,    // >
    LessEq,     // <=
    GreaterEq,  // >=
    And,        // &&
    Or,         // ||
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Neg,    // -
    Not,    // !
}
