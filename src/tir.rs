use std::{
    collections::BTreeMap,
    fmt::{Display, Error, Formatter},
    fs::read_to_string,
    path::PathBuf,
    process::exit,
};

use crate::{
    hir::{
        HirConstant, HirDeclaration, HirExpression, HirFunction, HirProgram, HirStatement,
        HirStructure, HirType,
    },
    parse, Identifier, StringLiteral, Target,
};

#[derive(Clone, Debug)]
pub enum TirError {
    InvalidCopyTypeSignature(Identifier),
    InvalidDropTypeSignature(Identifier),
    ExplicitCopy,
}

impl Display for TirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::InvalidCopyTypeSignature(type_name) => write!(
                f,
                "invalid copy constructor type signature for type '{}'",
                type_name
            ),
            Self::InvalidDropTypeSignature(type_name) => write!(
                f,
                "invalid drop destructor type signature for type '{}'",
                type_name
            ),
            Self::ExplicitCopy => write!(f, "cannot explicitly call copy constructors"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TirProgram(Vec<TirDeclaration>, i32);

impl TirProgram {
    pub fn new(decls: Vec<TirDeclaration>, memory_size: i32) -> Self {
        Self(decls, memory_size)
    }

    pub fn compile(&self, target: &impl Target) -> Result<HirProgram, TirError> {
        let mut hir_decls = vec![];
        for decl in &self.0 {
            hir_decls.push(decl.to_hir_decl(target)?);
        }

        Ok(HirProgram::new(hir_decls, self.1))
    }
}

#[derive(Clone, Debug)]
pub enum TirDeclaration {
    DocumentHeader(String),
    Constant(Option<String>, Identifier, TirConstant),
    Function(TirFunction),
    Structure(TirStructure),
    Assert(TirConstant),
    If(TirConstant, TirProgram),
    IfElse(TirConstant, TirProgram, TirProgram),
    Error(String),
    Extern(String),
    Include(String),
    Memory(i32),
    RequireStd,
    NoStd,
}

impl TirDeclaration {
    fn to_hir_decl(&self, target: &impl Target) -> Result<HirDeclaration, TirError> {
        Ok(match self {
            Self::DocumentHeader(header) => HirDeclaration::DocumentHeader(header.clone()),
            Self::Constant(doc, name, constant) => HirDeclaration::Constant(doc.clone(), name.clone(), constant.to_hir_const()?),
            Self::Function(func) => HirDeclaration::Function(func.to_hir_fn()?),
            Self::Structure(structure) => HirDeclaration::Structure(structure.clone().to_hir_struct()?),
            
            Self::Assert(constant) => HirDeclaration::Assert(
                constant.to_hir_const()?
            ),
            
            Self::Error(msg) => HirDeclaration::Error(
                msg.clone()
            ),
            
            Self::Extern(file) => HirDeclaration::Extern(
                file.clone()
            ),
            
            Self::Include(file) => HirDeclaration::Include(
                file.clone()
            ),
            
            Self::Memory(n) => HirDeclaration::Memory(
                *n
            ),
            
            Self::RequireStd => HirDeclaration::RequireStd,
            Self::NoStd => HirDeclaration::NoStd,
            
            Self::If(constant, program) => HirDeclaration::If(
                constant.to_hir_const()?,
                program.compile(target)?
            ),
            
            Self::IfElse(constant, then_prog, else_prog) => HirDeclaration::IfElse(
                constant.to_hir_const()?,
                then_prog.compile(target)?,
                else_prog.compile(target)?
            ),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TirType {
    Pointer(Box<Self>),
    Void,
    Float,
    Boolean,
    Character,
    Structure(Identifier),
}

impl TirType {
    fn refer(&self) -> Self {
        Self::Pointer(Box::new(self.clone()))
    }

    fn to_hir_type(&self) -> Result<HirType, TirError> {
        Ok(match self {
            Self::Pointer(inner) => HirType::Pointer(Box::new(inner.to_hir_type()?)),
            Self::Void => HirType::Void,
            Self::Float => HirType::Float,
            Self::Boolean => HirType::Boolean,
            Self::Character => HirType::Character,
            Self::Structure(name) => HirType::Structure(name.clone()),
        })
    }
}

#[derive(Clone, Debug)]
pub struct TirFunction {
    doc: Option<String>,
    name: Identifier,
    args: Vec<(Identifier, TirType)>,
    return_type: TirType,
    body: Vec<TirStatement>,
}

impl TirFunction {
    pub fn new(
        doc: Option<String>,
        name: Identifier,
        args: Vec<(Identifier, TirType)>,
        return_type: TirType,
        body: Vec<TirStatement>,
    ) -> Self {
        Self {
            doc,
            name,
            args,
            return_type,
            body,
        }
    }

    fn member_method(
        self_type: &Identifier,
        previous_member_types: &Vec<TirType>,
        member_name: &Identifier,
        member_type: &TirType,
    ) -> Self {
        let mut fn_return = TirExpression::Variable(Identifier::from("self"));
        for t in previous_member_types {
            fn_return = TirExpression::Add(
                Box::new(fn_return.clone()),
                Box::new(TirExpression::SizeOf(t.clone())),
            );
        }

        Self::new(
            None,
            member_name.clone(),
            vec![(
                Identifier::from("self"),
                TirType::Pointer(Box::new(TirType::Structure(self_type.clone()))),
            )],
            member_type.refer().clone(),
            vec![TirStatement::Return(vec![TirExpression::TypeCast(
                Box::new(fn_return),
                member_type.refer().clone(),
            )])],
        )
    }

    fn copy_constructor(structure: &Identifier) -> Self {
        let struct_t = TirType::Structure(structure.clone());
        Self::new(
            None,
            Identifier::from("copy"),
            vec![(Identifier::from("self"), struct_t.refer())],
            struct_t,
            vec![TirStatement::Return(vec![TirExpression::Move(Box::new(
                TirExpression::Deref(Box::new(TirExpression::Variable(Identifier::from("self")))),
            ))])],
        )
    }

    fn drop_destructor(structure: &Identifier) -> Self {
        let struct_t = TirType::Structure(structure.clone());
        Self::new(
            None,
            Identifier::from("drop"),
            vec![(Identifier::from("self"), struct_t.refer())],
            TirType::Void,
            vec![],
        )
    }

    fn is_valid_copy(&self, structure: &Identifier) -> Result<bool, TirError> {
        if &self.name == "copy" {
            let struct_t = TirType::Structure(structure.clone());
            if self.args.len() == 1
                && self.args[0].1 == struct_t.refer()
                && self.return_type == struct_t
            {
                return Ok(true);
            } else {
                return Err(TirError::InvalidCopyTypeSignature(structure.clone()));
            }
        }
        return Ok(false);
    }

    fn is_valid_drop(&self, structure: &Identifier) -> Result<bool, TirError> {
        if &self.name == "drop" {
            let struct_t = TirType::Structure(structure.clone());
            if self.args.len() == 1
                && self.args[0].1 == struct_t.refer()
                && self.return_type == TirType::Void
            {
                return Ok(true);
            } else {
                return Err(TirError::InvalidDropTypeSignature(structure.clone()));
            }
        }
        return Ok(false);
    }

    fn to_hir_fn(&self) -> Result<HirFunction, TirError> {
        let mut args = vec![];
        for (arg, t) in &self.args {
            args.push((arg.clone(), t.to_hir_type()?))
        }

        let mut body = vec![];
        for stmt in &self.body {
            body.push(stmt.to_hir_stmt()?)
        }

        Ok(HirFunction::new(
            self.doc.clone(),
            self.name.clone(),
            args,
            self.return_type.to_hir_type()?,
            body,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct TirStructure {
    doc: Option<String>,
    name: Identifier,
    members: Vec<(Identifier, TirType)>,
    methods: Vec<TirFunction>,
    default_copy: bool,
    default_drop: bool,
}

impl TirStructure {
    pub fn new(
        doc: Option<String>,
        name: Identifier,
        members: Vec<(Identifier, TirType)>,
        methods: Vec<TirFunction>,
    ) -> Self {
        Self {
            doc,
            name,
            members,
            methods,
            default_copy: false,
            default_drop: false,
        }
    }

    fn to_hir_struct(&mut self) -> Result<HirStructure, TirError> {
        self.add_copy_and_drop()?;

        let mut previous_member_types = vec![];
        let mut size = HirConstant::Float(0.0);
        let mut methods = vec![];
        for (name, t) in &self.members {
            methods.push(
                TirFunction::member_method(&self.name, &previous_member_types, name, t)
                    .to_hir_fn()?,
            );
            size = HirConstant::Add(
                Box::new(size.clone()),
                Box::new(HirConstant::SizeOf(t.to_hir_type()?)),
            );
            previous_member_types.push(t.clone())
        }

        for method in &self.methods {
            methods.push(method.to_hir_fn()?)
        }

        Ok(HirStructure::new(
            self.doc.clone(),
            self.name.clone(),
            size,
            methods,
        ))
    }

    fn add_copy_and_drop(&mut self) -> Result<(), TirError> {
        let mut has_copy = false;
        let mut has_drop = false;
        for method in &self.methods {
            if method.is_valid_copy(&self.name)? {
                has_copy = true;
            } else if method.is_valid_drop(&self.name)? {
                has_drop = true;
            }
        }

        if !has_copy {
            self.methods.push(TirFunction::copy_constructor(&self.name));
            // If the user does not specify a `copy` method, specify that
            // the `copy` method is a default.
            self.default_copy = true;
        }

        if !has_drop {
            self.methods.push(TirFunction::drop_destructor(&self.name));
            // If the user does not specify a `drop` method, specify that
            // the `drop` method is a default.
            self.default_drop = true;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum TirConstant {
    Float(f64),
    Character(char),
    True,
    False,

    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),

    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),

    Greater(Box<Self>, Box<Self>),
    Less(Box<Self>, Box<Self>),
    GreaterEqual(Box<Self>, Box<Self>),
    LessEqual(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),

    Constant(Identifier),
    IsDefined(String),
    SizeOf(TirType),
    Not(Box<Self>),
}

impl TirConstant {
    pub fn to_hir_const(&self) -> Result<HirConstant, TirError> {
        Ok(match self {
            Self::Float(n) => HirConstant::Float(*n),
            Self::Character(ch) => HirConstant::Character(*ch),
            Self::True => HirConstant::True,
            Self::False => HirConstant::False,

            Self::Add(lhs, rhs) => {
                HirConstant::Add(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Subtract(lhs, rhs) => {
                HirConstant::Subtract(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Multiply(lhs, rhs) => {
                HirConstant::Multiply(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Divide(lhs, rhs) => {
                HirConstant::Divide(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }

            Self::Greater(lhs, rhs) => {
                HirConstant::Greater(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Less(lhs, rhs) => {
                HirConstant::Less(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::GreaterEqual(lhs, rhs) => HirConstant::GreaterEqual(
                Box::new(lhs.to_hir_const()?),
                Box::new(rhs.to_hir_const()?),
            ),
            Self::LessEqual(lhs, rhs) => {
                HirConstant::LessEqual(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Equal(lhs, rhs) => {
                HirConstant::Equal(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::NotEqual(lhs, rhs) => {
                HirConstant::NotEqual(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }

            Self::Constant(name) => HirConstant::Constant(name.clone()),
            Self::IsDefined(name) => HirConstant::IsDefined(name.clone()),
            Self::SizeOf(t) => HirConstant::SizeOf(t.to_hir_type()?),

            Self::And(lhs, rhs) => {
                HirConstant::And(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Or(lhs, rhs) => {
                HirConstant::Or(Box::new(lhs.to_hir_const()?), Box::new(rhs.to_hir_const()?))
            }
            Self::Not(expr) => HirConstant::Not(Box::new(expr.to_hir_const()?)),
        })
    }
}

#[derive(Clone, Debug)]
pub enum TirStatement {
    /// An HIR let expression with a manually assigned type
    Define(Identifier, TirType, TirExpression),
    /// An HIR let expression with an automatically assigned type
    AutoDefine(Identifier, TirExpression),
    /// A variable assignment
    AssignVariable(Identifier, TirExpression),
    AddAssignVariable(Identifier, TirExpression),
    SubtractAssignVariable(Identifier, TirExpression),
    MultiplyAssignVariable(Identifier, TirExpression),
    DivideAssignVariable(Identifier, TirExpression),
    /// An assignment to a dereferenced address
    AssignAddress(TirExpression, TirExpression),
    AddAssignAddress(TirExpression, TirExpression),
    SubtractAssignAddress(TirExpression, TirExpression),
    MultiplyAssignAddress(TirExpression, TirExpression),
    DivideAssignAddress(TirExpression, TirExpression),

    /// An HIR for loop `for (let i=0; i<10; i=i+1) {...}`
    For(Box<Self>, TirExpression, Box<Self>, Vec<Self>),
    /// An HIR for loop `for i in 0..10 {...}`
    ForRange(
        Identifier,
        Box<TirExpression>,
        Box<TirExpression>,
        Vec<Self>,
    ),

    /// An HIR while loop
    While(TirExpression, Vec<Self>),
    /// An HIR if statement
    If(TirExpression, Vec<Self>),
    /// An HIR if statement with an else clause
    IfElse(TirExpression, Vec<Self>, Vec<Self>),
    /// An HIR if statement with an else clause
    IfElifElse(
        TirExpression,
        Vec<Self>,
        Vec<(TirExpression, Vec<Self>)>,
        Vec<Self>,
    ),

    /// An HIR free statement to deallocate memory
    Free(TirExpression, TirExpression),
    /// Return one or more values at the end of a function
    Return(Vec<TirExpression>),

    /// Any expression
    Expression(TirExpression),
}

impl TirStatement {
    fn to_hir_stmt(&self) -> Result<HirStatement, TirError> {
        Ok(match self {
            Self::Define(name, t, expr) => {
                HirStatement::Define(name.clone(), t.to_hir_type()?, expr.to_hir_expr()?)
            }
            Self::AutoDefine(name, expr) => {
                HirStatement::AutoDefine(name.clone(), expr.to_hir_expr()?)
            }
            Self::AssignVariable(name, expr) => {
                HirStatement::AssignVariable(name.clone(), expr.to_hir_expr()?)
            }
            Self::AddAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Add(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::SubtractAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Subtract(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::MultiplyAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Multiply(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::DivideAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Divide(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::AssignAddress(addr, expr) => {
                HirStatement::AssignAddress(addr.to_hir_expr()?, expr.to_hir_expr()?)
            }
            Self::AddAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr()?,
                HirExpression::Add(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr()?))),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::SubtractAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr()?,
                HirExpression::Subtract(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr()?))),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::MultiplyAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr()?,
                HirExpression::Multiply(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr()?))),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),
            Self::DivideAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr()?,
                HirExpression::Divide(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr()?))),
                    Box::new(expr.to_hir_expr()?),
                ),
            ),

            Self::For(pre, cond, post, body) => HirStatement::For(
                Box::new(pre.to_hir_stmt()?),
                cond.to_hir_expr()?,
                Box::new(post.to_hir_stmt()?),
                {
                    let mut result = vec![];
                    for stmt in body {
                        result.push(stmt.to_hir_stmt()?)
                    }
                    result
                },
            ),

            Self::ForRange(var, from, to, body) => HirStatement::For(
                Box::new(HirStatement::Define(
                    var.clone(),
                    HirType::Float,
                    from.to_hir_expr()?,
                )),
                HirExpression::Less(
                    Box::new(HirExpression::Variable(var.clone())),
                    Box::new(to.to_hir_expr()?),
                ),
                Box::new(HirStatement::AssignVariable(
                    var.clone(),
                    HirExpression::Add(
                        Box::new(HirExpression::Variable(var.clone())),
                        Box::new(HirExpression::Constant(HirConstant::Float(1.0))),
                    ),
                )),
                {
                    let mut result = vec![];
                    for stmt in body {
                        result.push(stmt.to_hir_stmt()?)
                    }
                    result
                },
            ),

            Self::While(cond, body) => HirStatement::While(cond.to_hir_expr()?, {
                let mut result = vec![];
                for stmt in body {
                    result.push(stmt.to_hir_stmt()?)
                }
                result
            }),

            Self::If(cond, body) => HirStatement::If(cond.to_hir_expr()?, {
                let mut result = vec![];
                for stmt in body {
                    result.push(stmt.to_hir_stmt()?)
                }
                result
            }),

            Self::IfElse(cond, then_body, else_body) => HirStatement::IfElse(
                cond.to_hir_expr()?,
                {
                    let mut result = vec![];
                    for stmt in then_body {
                        result.push(stmt.to_hir_stmt()?)
                    }
                    result
                },
                {
                    let mut result = vec![];
                    for stmt in else_body {
                        result.push(stmt.to_hir_stmt()?)
                    }
                    result
                },
            ),

            Self::IfElifElse(cond, then_body, elifs, else_body) => {
                let mut else_branch = else_body.clone();
                for (elif_cond, elif_body) in elifs {
                    else_branch = vec![Self::IfElse(
                        elif_cond.clone(),
                        elif_body.clone(),
                        else_branch.clone(),
                    )];
                }
                Self::IfElse(cond.clone(), then_body.clone(), else_branch).to_hir_stmt()?
            }

            Self::Free(addr, size) => HirStatement::Free(addr.to_hir_expr()?, size.to_hir_expr()?),
            Self::Return(exprs) => HirStatement::Return({
                let mut result = vec![];
                for expr in exprs {
                    result.push(expr.to_hir_expr()?)
                }
                result
            }),

            Self::Expression(expr) => HirStatement::Expression(expr.to_hir_expr()?),
        })
    }
}

#[derive(Clone, Debug)]
pub enum TirExpression {
    SizeOf(TirType),
    Constant(TirConstant),
    Move(Box<Self>),

    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),

    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),

    Greater(Box<Self>, Box<Self>),
    Less(Box<Self>, Box<Self>),
    GreaterEqual(Box<Self>, Box<Self>),
    LessEqual(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),

    Refer(Identifier),
    Deref(Box<Self>),

    Void,
    True,
    False,
    Character(char),
    String(StringLiteral),
    Variable(Identifier),

    TypeCast(Box<Self>, TirType),
    Alloc(Box<Self>),

    Call(Identifier, Vec<Self>),
    ForeignCall(Identifier, Vec<Self>),
    Method(Box<Self>, Identifier, Vec<Self>),
    Index(Box<Self>, Box<Self>),
}

impl TirExpression {
    pub fn to_hir_expr(&self) -> Result<HirExpression, TirError> {
        Ok(match self {
            Self::Void => HirExpression::Void,
            Self::True => HirExpression::True,
            Self::False => HirExpression::False,
            Self::Character(ch) => HirExpression::Character(*ch),
            Self::String(s) => HirExpression::String(s.clone()),
            Self::Variable(name) => HirExpression::Variable(name.clone()),

            Self::Move(expr) => HirExpression::Move(Box::new(expr.to_hir_expr()?)),
            Self::SizeOf(t) => HirExpression::SizeOf(t.to_hir_type()?),
            Self::Constant(constant) => HirExpression::Constant(constant.to_hir_const()?),

            Self::And(lhs, rhs) => {
                HirExpression::And(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Or(lhs, rhs) => {
                HirExpression::Or(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Not(expr) => HirExpression::Not(Box::new(expr.to_hir_expr()?)),

            Self::Add(lhs, rhs) => {
                HirExpression::Add(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Subtract(lhs, rhs) => {
                HirExpression::Subtract(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Multiply(lhs, rhs) => {
                HirExpression::Multiply(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Divide(lhs, rhs) => {
                HirExpression::Divide(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Greater(lhs, rhs) => {
                HirExpression::Greater(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Less(lhs, rhs) => {
                HirExpression::Less(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::GreaterEqual(lhs, rhs) => HirExpression::GreaterEqual(
                Box::new(lhs.to_hir_expr()?),
                Box::new(rhs.to_hir_expr()?),
            ),

            Self::LessEqual(lhs, rhs) => {
                HirExpression::LessEqual(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Equal(lhs, rhs) => {
                HirExpression::Equal(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::NotEqual(lhs, rhs) => {
                HirExpression::NotEqual(Box::new(lhs.to_hir_expr()?), Box::new(rhs.to_hir_expr()?))
            }

            Self::Refer(name) => HirExpression::Refer(name.clone()),
            Self::Deref(ptr) => HirExpression::Deref(Box::new(ptr.to_hir_expr()?)),

            Self::TypeCast(expr, t) => {
                HirExpression::TypeCast(Box::new(expr.to_hir_expr()?), t.to_hir_type()?)
            }

            Self::Alloc(expr) => HirExpression::Alloc(Box::new(expr.to_hir_expr()?)),

            Self::Call(name, args) => HirExpression::Call(name.clone(), {
                let mut result = vec![];
                for arg in args {
                    result.push(arg.to_hir_expr()?)
                }
                result
            }),

            Self::ForeignCall(name, args) => HirExpression::ForeignCall(name.clone(), {
                let mut result = vec![];
                for arg in args {
                    result.push(arg.to_hir_expr()?)
                }
                result
            }),

            Self::Method(instance, name, args) => {
                if name == "copy" {
                    return Err(TirError::ExplicitCopy);
                }

                HirExpression::Method(Box::new(instance.to_hir_expr()?), name.clone(), {
                    let mut result = vec![];
                    for arg in args {
                        result.push(arg.to_hir_expr()?)
                    }
                    result
                })
            }

            Self::Index(ptr, idx) => {
                HirExpression::Index(Box::new(ptr.to_hir_expr()?), Box::new(idx.to_hir_expr()?))
            }
        })
    }
}
