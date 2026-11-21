// SPDX-License-Identifier: GPL-3.0-or-later
//! The syntax tree of a program: what the parser makes and the report walks. It holds the shape of
//! the program and the place of each part, and no value is computed from it.

/// A place in the program, from one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Pos {
    /// The line.
    pub line: usize,
    /// The column.
    pub col: usize,
}

/// A directive at the top of a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    /// `include`, `usage` or `require`.
    pub name: String,
    /// The texts or the number that follow it.
    pub args: Vec<String>,
    /// Where it stands.
    pub pos: Pos,
}

/// A declarator: a name with its brackets and its initial value.
#[derive(Debug, Clone, PartialEq)]
pub struct Declarator {
    /// The name.
    pub name: String,
    /// True for a name followed by `[]` or `[size]`.
    pub array: bool,
    /// The size inside the brackets, when there is one.
    pub size: Option<Expr>,
    /// The initial value: one expression or the list in braces.
    pub init: Option<Init>,
    /// Where it stands.
    pub pos: Pos,
}

/// The initial value of a declarator.
#[derive(Debug, Clone, PartialEq)]
pub enum Init {
    /// `= expression`.
    Value(Expr),
    /// `= { a, b, c }`.
    List(Vec<Expr>),
    /// `= call(...) { statements }`: the value of a dialog, which is the button that closed it.
    Block {
        /// The call.
        call: Expr,
        /// The block.
        body: Vec<Stmt>,
    },
}

/// A declaration of one type and one or more names.
#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    /// The type: `char`, `int`, `real`, `string` or a name that starts with `UL_`.
    pub ty: String,
    /// The names declared.
    pub names: Vec<Declarator>,
}

/// A parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The type.
    pub ty: String,
    /// The name.
    pub name: String,
    /// True for a name followed by `[]`.
    pub array: bool,
}

/// A function the program defines.
#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    /// The type it returns.
    pub ret: String,
    /// The name.
    pub name: String,
    /// The parameters.
    pub params: Vec<Param>,
    /// The statements of its body.
    pub body: Vec<Stmt>,
    /// Where it stands.
    pub pos: Pos,
}

/// A statement with its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// What it is.
    pub kind: StmtKind,
    /// Where it starts.
    pub pos: Pos,
}

/// The kinds of statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `;`
    Empty,
    /// A declaration.
    Decl(Decl),
    /// An expression and `;`.
    Expr(Expr),
    /// A call followed by a block: a walk over the objects of the design, an output file or a
    /// dialog. The block is run for each object.
    Loop {
        /// The call: `board(B)`, `B.elements(E)`, `output("file")`, `dlgDialog("title")`.
        call: Expr,
        /// The block.
        body: Vec<Stmt>,
    },
    /// `{ ... }`
    Block(Vec<Stmt>),
    /// `if (c) a else b`
    If {
        /// The condition.
        cond: Expr,
        /// The statement when true.
        then: Box<Stmt>,
        /// The statement when false.
        otherwise: Option<Box<Stmt>>,
    },
    /// `while (c) body`
    While {
        /// The condition.
        cond: Expr,
        /// The body.
        body: Box<Stmt>,
    },
    /// `do body while (c);`
    Do {
        /// The body.
        body: Box<Stmt>,
        /// The condition.
        cond: Expr,
    },
    /// `for (init; cond; step) body`
    For {
        /// The first part: a declaration or an expression.
        init: Option<Box<Stmt>>,
        /// The condition.
        cond: Option<Expr>,
        /// The step.
        step: Option<Expr>,
        /// The body.
        body: Box<Stmt>,
    },
    /// `switch (e) { ... }`
    Switch {
        /// The value looked at.
        value: Expr,
        /// The statements of the block, with the `case` and `default` marks among them.
        body: Vec<Stmt>,
    },
    /// `case e:`
    Case(Expr),
    /// `default:`
    Default,
    /// `break;`
    Break,
    /// `continue;`
    Continue,
    /// `return;` or `return e;`
    Return(Option<Expr>),
}

/// An expression with its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// What it is.
    pub kind: ExprKind,
    /// Where it starts.
    pub pos: Pos,
}

/// The kinds of expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A whole number (its text).
    Int(String),
    /// A number with a point or an exponent (its text).
    Real(String),
    /// A character.
    Char(String),
    /// A text (the adjacent texts are joined).
    Str(String),
    /// A name.
    Name(String),
    /// A call: `f(a, b)` or `X.loop(E)`.
    Call {
        /// What is called: a name or a member.
        callee: Box<Expr>,
        /// The arguments.
        args: Vec<Expr>,
    },
    /// `a[i]`
    Index {
        /// The array.
        base: Box<Expr>,
        /// The index.
        index: Box<Expr>,
    },
    /// `a.name`
    Member {
        /// The object.
        base: Box<Expr>,
        /// The member.
        name: String,
    },
    /// A prefix operator: `- ! ~ ++ --`.
    Unary {
        /// The operator.
        op: String,
        /// The operand.
        operand: Box<Expr>,
    },
    /// A suffix operator: `++ --`.
    Postfix {
        /// The operator.
        op: String,
        /// The operand.
        operand: Box<Expr>,
    },
    /// A binary operator, the comma included.
    Binary {
        /// The operator.
        op: String,
        /// The left side.
        left: Box<Expr>,
        /// The right side.
        right: Box<Expr>,
    },
    /// `a = b`, `a += b` and the like.
    Assign {
        /// The operator.
        op: String,
        /// What is assigned to.
        target: Box<Expr>,
        /// The value.
        value: Box<Expr>,
    },
    /// `c ? a : b`
    Cond {
        /// The condition.
        cond: Box<Expr>,
        /// The value when true.
        then: Box<Expr>,
        /// The value when false.
        otherwise: Box<Expr>,
    },
}

/// A program read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    /// The directives, in order.
    pub directives: Vec<Directive>,
    /// The functions the program defines, in order.
    pub functions: Vec<Func>,
    /// The statements outside any function, in order.
    pub body: Vec<Stmt>,
}

impl Program {
    /// How many statements the program has, counting the ones inside blocks and functions.
    pub fn statement_count(&self) -> usize {
        fn count(list: &[Stmt]) -> usize {
            list.iter()
                .map(|s| {
                    1 + match &s.kind {
                        StmtKind::Decl(d) => d
                            .names
                            .iter()
                            .map(|n| match &n.init {
                                Some(Init::Block { body, .. }) => count(body),
                                _ => 0,
                            })
                            .sum(),
                        StmtKind::Loop { body, .. }
                        | StmtKind::Block(body)
                        | StmtKind::Switch { body, .. } => count(body),
                        StmtKind::If {
                            then, otherwise, ..
                        } => {
                            count(std::slice::from_ref(then))
                                + otherwise
                                    .as_ref()
                                    .map_or(0, |o| count(std::slice::from_ref(o)))
                        }
                        StmtKind::While { body, .. } | StmtKind::Do { body, .. } => {
                            count(std::slice::from_ref(body))
                        }
                        StmtKind::For { init, body, .. } => {
                            init.as_ref().map_or(0, |i| count(std::slice::from_ref(i)))
                                + count(std::slice::from_ref(body))
                        }
                        _ => 0,
                    }
                })
                .sum()
        }
        count(&self.body)
            + self
                .functions
                .iter()
                .map(|f| 1 + count(&f.body))
                .sum::<usize>()
    }
}
