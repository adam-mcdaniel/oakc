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
    StructureNotDefined(Identifier),
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
            Self::StructureNotDefined(type_name) => {
                write!(f, "type '{}' is not defined", type_name)
            }
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

    pub fn compile(&self) -> Result<HirProgram, TirError> {
        let mut hir_decls = vec![];
        for decl in &self.0 {
            hir_decls.push(decl.to_hir_decl(&self.0)?);
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
    fn to_hir_decl(&self, decls: &Vec<TirDeclaration>) -> Result<HirDeclaration, TirError> {
        Ok(match self {
            Self::DocumentHeader(header) => HirDeclaration::DocumentHeader(header.clone()),
            Self::Constant(doc, name, constant) => {
                HirDeclaration::Constant(doc.clone(), name.clone(), constant.to_hir_const(decls)?)
            }
            Self::Function(func) => HirDeclaration::Function(func.to_hir_fn(decls)?),
            Self::Structure(structure) => {
                HirDeclaration::Structure(structure.clone().to_hir_struct(decls)?)
            }

            Self::Assert(constant) => HirDeclaration::Assert(constant.to_hir_const(decls)?),

            Self::Error(msg) => HirDeclaration::Error(msg.clone()),

            Self::Extern(file) => HirDeclaration::Extern(file.clone()),

            Self::Include(file) => HirDeclaration::Include(file.clone()),

            Self::Memory(n) => HirDeclaration::Memory(*n),

            Self::RequireStd => HirDeclaration::RequireStd,
            Self::NoStd => HirDeclaration::NoStd,

            Self::If(constant, program) => {
                HirDeclaration::If(constant.to_hir_const(decls)?, program.compile()?)
            }

            Self::IfElse(constant, then_prog, else_prog) => HirDeclaration::IfElse(
                constant.to_hir_const(decls)?,
                then_prog.compile()?,
                else_prog.compile()?,
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
    fn is_primitive(&self) -> bool {
        match self {
            Self::Structure(_) => false,
            Self::Pointer(ptr) => ptr.is_primitive(),
            _ => true,
        }
    }

    fn is_movable(&self, decls: &Vec<TirDeclaration>) -> Result<bool, TirError> {
        if let Self::Structure(name) = self {
            for decl in decls {
                if let TirDeclaration::Structure(structure) = decl {
                    if name == structure.get_name() {
                        return Ok(structure.is_movable(decls)?);
                    }
                }
            }
            return Err(TirError::StructureNotDefined(name.clone()));
        } else {
            return Ok(true);
        }
    }

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

    fn copy_constructor(members: &Vec<(Identifier, TirType)>, structure: &Identifier) -> Self {
        let struct_t = TirType::Structure(structure.clone());
        let mut result = vec![];

        if members.len() == 1 {
            let member_name = members[0].0.clone();

            result = vec![TirExpression::TypeCast(
                Box::new(TirExpression::Deref(Box::new(TirExpression::Method(
                    Box::new(TirExpression::Variable(Identifier::from("self"))),
                    member_name,
                    vec![],
                )))),
                TirType::Structure(structure.clone()),
            )]
        } else {
            for (member, _) in members {
                result.push(TirExpression::Deref(Box::new(TirExpression::Method(
                    Box::new(TirExpression::Variable(Identifier::from("self"))),
                    member.clone(),
                    vec![],
                ))))
            }
        }

        Self::new(
            None,
            Identifier::from("copy"),
            vec![(Identifier::from("self"), struct_t.refer())],
            struct_t,
            vec![TirStatement::Return(result)],
        )
    }

    fn drop_destructor(members: &Vec<(Identifier, TirType)>, structure: &Identifier) -> Self {
        let struct_t = TirType::Structure(structure.clone());
        let mut result = vec![];
        for (member, t) in members {
            if !t.is_primitive() {
                result.push(TirStatement::Expression(TirExpression::Method(
                    Box::new(TirExpression::Method(
                        Box::new(TirExpression::Variable(Identifier::from("self"))),
                        member.clone(),
                        vec![],
                    )),
                    Identifier::from("drop"),
                    vec![],
                )))
            }
        }

        Self::new(
            None,
            Identifier::from("drop"),
            vec![(Identifier::from("self"), struct_t.refer())],
            TirType::Void,
            result,
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

    fn to_hir_fn(&self, decls: &Vec<TirDeclaration>) -> Result<HirFunction, TirError> {
        let mut args = vec![];
        for (arg, t) in &self.args {
            args.push((arg.clone(), t.to_hir_type()?))
        }

        let mut body = vec![];
        for stmt in &self.body {
            body.push(stmt.to_hir_stmt(decls)?)
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
        }
    }

    fn get_name(&self) -> &Identifier {
        &self.name
    }

    fn is_movable(&self, decls: &Vec<TirDeclaration>) -> Result<bool, TirError> {
        let mut default_copy = true;
        let mut default_drop = true;
        for method in &self.methods {
            if method.is_valid_copy(&self.name)? {
                default_copy = false;
            }

            if method.is_valid_drop(&self.name)? {
                default_drop = false;
            }
        }

        let mut is_movable = default_copy && default_drop;
        for (_, t) in &self.members {
            if !t.is_movable(decls)? {
                is_movable = false;
            }
        }
        Ok(is_movable)
    }

    fn to_hir_struct(&mut self, decls: &Vec<TirDeclaration>) -> Result<HirStructure, TirError> {
        let is_movable = self.is_movable(decls)?;
        self.add_copy_and_drop()?;

        let mut previous_member_types = vec![];
        let mut size = HirConstant::Float(0.0);
        let mut methods = vec![];
        for (name, t) in &self.members {
            methods.push(
                TirFunction::member_method(&self.name, &previous_member_types, name, t)
                    .to_hir_fn(decls)?,
            );
            size = HirConstant::Add(
                Box::new(size.clone()),
                Box::new(HirConstant::SizeOf(t.to_hir_type()?)),
            );
            previous_member_types.push(t.clone())
        }

        for method in &self.methods {
            methods.push(method.to_hir_fn(decls)?)
        }

        Ok(HirStructure::new(
            self.doc.clone(),
            self.name.clone(),
            size,
            methods,
            is_movable,
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
            self.methods
                .push(TirFunction::copy_constructor(&self.members, &self.name));
        }

        if !has_drop {
            self.methods
                .push(TirFunction::drop_destructor(&self.members, &self.name));
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
    IsMovable(TirType),
    SizeOf(TirType),
    Not(Box<Self>),
}

impl TirConstant {
    pub fn to_hir_const(&self, decls: &Vec<TirDeclaration>) -> Result<HirConstant, TirError> {
        Ok(match self {
            Self::Float(n) => HirConstant::Float(*n),
            Self::Character(ch) => HirConstant::Character(*ch),
            Self::True => HirConstant::True,
            Self::False => HirConstant::False,

            Self::Add(lhs, rhs) => HirConstant::Add(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Subtract(lhs, rhs) => HirConstant::Subtract(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Multiply(lhs, rhs) => HirConstant::Multiply(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Divide(lhs, rhs) => HirConstant::Divide(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),

            Self::Greater(lhs, rhs) => HirConstant::Greater(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Less(lhs, rhs) => HirConstant::Less(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::GreaterEqual(lhs, rhs) => HirConstant::GreaterEqual(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::LessEqual(lhs, rhs) => HirConstant::LessEqual(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Equal(lhs, rhs) => HirConstant::Equal(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::NotEqual(lhs, rhs) => HirConstant::NotEqual(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),

            Self::Constant(name) => HirConstant::Constant(name.clone()),
            Self::IsDefined(name) => HirConstant::IsDefined(name.clone()),
            Self::IsMovable(t) => HirConstant::Float(t.is_movable(decls)? as i32 as f64),
            Self::SizeOf(t) => HirConstant::SizeOf(t.to_hir_type()?),

            Self::And(lhs, rhs) => HirConstant::And(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Or(lhs, rhs) => HirConstant::Or(
                Box::new(lhs.to_hir_const(decls)?),
                Box::new(rhs.to_hir_const(decls)?),
            ),
            Self::Not(expr) => HirConstant::Not(Box::new(expr.to_hir_const(decls)?)),
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
    ForRange(Identifier, TirExpression, TirExpression, Vec<Self>),

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
    fn to_hir_stmt(&self, decls: &Vec<TirDeclaration>) -> Result<HirStatement, TirError> {
        Ok(match self {
            Self::Define(name, t, expr) => {
                HirStatement::Define(name.clone(), t.to_hir_type()?, expr.to_hir_expr(decls)?)
            }
            Self::AutoDefine(name, expr) => {
                HirStatement::AutoDefine(name.clone(), expr.to_hir_expr(decls)?)
            }
            Self::AssignVariable(name, expr) => {
                HirStatement::AssignVariable(name.clone(), expr.to_hir_expr(decls)?)
            }
            Self::AddAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Add(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::SubtractAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Subtract(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::MultiplyAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Multiply(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::DivideAssignVariable(name, expr) => HirStatement::AssignVariable(
                name.clone(),
                HirExpression::Divide(
                    Box::new(HirExpression::Variable(name.clone())),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::AssignAddress(addr, expr) => {
                HirStatement::AssignAddress(addr.to_hir_expr(decls)?, expr.to_hir_expr(decls)?)
            }
            Self::AddAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr(decls)?,
                HirExpression::Add(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr(decls)?))),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::SubtractAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr(decls)?,
                HirExpression::Subtract(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr(decls)?))),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::MultiplyAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr(decls)?,
                HirExpression::Multiply(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr(decls)?))),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),
            Self::DivideAssignAddress(addr, expr) => HirStatement::AssignAddress(
                addr.to_hir_expr(decls)?,
                HirExpression::Divide(
                    Box::new(HirExpression::Deref(Box::new(addr.to_hir_expr(decls)?))),
                    Box::new(expr.to_hir_expr(decls)?),
                ),
            ),

            Self::For(pre, cond, post, body) => HirStatement::For(
                Box::new(pre.to_hir_stmt(decls)?),
                cond.to_hir_expr(decls)?,
                Box::new(post.to_hir_stmt(decls)?),
                {
                    let mut result = vec![];
                    for stmt in body {
                        result.push(stmt.to_hir_stmt(decls)?)
                    }
                    result
                },
            ),

            Self::ForRange(var, from, to, body) => HirStatement::For(
                Box::new(HirStatement::Define(
                    var.clone(),
                    HirType::Float,
                    from.to_hir_expr(decls)?,
                )),
                HirExpression::Less(
                    Box::new(HirExpression::Variable(var.clone())),
                    Box::new(to.to_hir_expr(decls)?),
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
                        result.push(stmt.to_hir_stmt(decls)?)
                    }
                    result
                },
            ),

            Self::While(cond, body) => HirStatement::While(cond.to_hir_expr(decls)?, {
                let mut result = vec![];
                for stmt in body {
                    result.push(stmt.to_hir_stmt(decls)?)
                }
                result
            }),

            Self::If(cond, body) => HirStatement::If(cond.to_hir_expr(decls)?, {
                let mut result = vec![];
                for stmt in body {
                    result.push(stmt.to_hir_stmt(decls)?)
                }
                result
            }),

            Self::IfElse(cond, then_body, else_body) => HirStatement::IfElse(
                cond.to_hir_expr(decls)?,
                {
                    let mut result = vec![];
                    for stmt in then_body {
                        result.push(stmt.to_hir_stmt(decls)?)
                    }
                    result
                },
                {
                    let mut result = vec![];
                    for stmt in else_body {
                        result.push(stmt.to_hir_stmt(decls)?)
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
                Self::IfElse(cond.clone(), then_body.clone(), else_branch).to_hir_stmt(decls)?
            }

            Self::Free(addr, size) => {
                HirStatement::Free(addr.to_hir_expr(decls)?, size.to_hir_expr(decls)?)
            }
            Self::Return(exprs) => HirStatement::Return({
                let mut result = vec![];
                for expr in exprs {
                    result.push(expr.to_hir_expr(decls)?)
                }
                result
            }),

            Self::Expression(expr) => HirStatement::Expression(expr.to_hir_expr(decls)?),
        })
    }
}

#[derive(Clone, Debug)]
pub enum TirExpression {
    IsMovable(TirType),
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
    pub fn to_hir_expr(&self, decls: &Vec<TirDeclaration>) -> Result<HirExpression, TirError> {
        Ok(match self {
            Self::IsMovable(t) => {
                if t.is_movable(decls)? {
                    HirExpression::True
                } else {
                    HirExpression::False
                }
            }
            Self::Void => HirExpression::Void,
            Self::True => HirExpression::True,
            Self::False => HirExpression::False,
            Self::Character(ch) => HirExpression::Character(*ch),
            Self::String(s) => HirExpression::String(s.clone()),
            Self::Variable(name) => HirExpression::Variable(name.clone()),

            Self::Move(expr) => HirExpression::Move(Box::new(expr.to_hir_expr(decls)?)),
            Self::SizeOf(t) => HirExpression::SizeOf(t.to_hir_type()?),
            Self::Constant(constant) => HirExpression::Constant(constant.to_hir_const(decls)?),

            Self::And(lhs, rhs) => HirExpression::And(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Or(lhs, rhs) => HirExpression::Or(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Not(expr) => HirExpression::Not(Box::new(expr.to_hir_expr(decls)?)),

            Self::Add(lhs, rhs) => HirExpression::Add(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Subtract(lhs, rhs) => HirExpression::Subtract(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Multiply(lhs, rhs) => HirExpression::Multiply(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Divide(lhs, rhs) => HirExpression::Divide(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Greater(lhs, rhs) => HirExpression::Greater(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Less(lhs, rhs) => HirExpression::Less(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::GreaterEqual(lhs, rhs) => HirExpression::GreaterEqual(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::LessEqual(lhs, rhs) => HirExpression::LessEqual(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Equal(lhs, rhs) => HirExpression::Equal(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::NotEqual(lhs, rhs) => HirExpression::NotEqual(
                Box::new(lhs.to_hir_expr(decls)?),
                Box::new(rhs.to_hir_expr(decls)?),
            ),

            Self::Refer(name) => HirExpression::Refer(name.clone()),
            Self::Deref(ptr) => HirExpression::Deref(Box::new(ptr.to_hir_expr(decls)?)),

            Self::TypeCast(expr, t) => {
                HirExpression::TypeCast(Box::new(expr.to_hir_expr(decls)?), t.to_hir_type()?)
            }

            Self::Alloc(expr) => HirExpression::Alloc(Box::new(expr.to_hir_expr(decls)?)),

            Self::Call(name, args) => HirExpression::Call(name.clone(), {
                let mut result = vec![];
                for arg in args {
                    result.push(arg.to_hir_expr(decls)?)
                }
                result
            }),

            Self::ForeignCall(name, args) => HirExpression::ForeignCall(name.clone(), {
                let mut result = vec![];
                for arg in args {
                    result.push(arg.to_hir_expr(decls)?)
                }
                result
            }),

            Self::Method(instance, name, args) => {
                if name == "copy" {
                    return Err(TirError::ExplicitCopy);
                }

                HirExpression::Method(Box::new(instance.to_hir_expr(decls)?), name.clone(), {
                    let mut result = vec![];
                    for arg in args {
                        result.push(arg.to_hir_expr(decls)?)
                    }
                    result
                })
            }

            Self::Index(ptr, idx) => HirExpression::Index(
                Box::new(ptr.to_hir_expr(decls)?),
                Box::new(idx.to_hir_expr(decls)?),
            ),
        })
    }
}
