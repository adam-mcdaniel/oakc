use std::{collections::BTreeMap, fs::read_to_string, path::PathBuf, process::exit};

use crate::{
    mir::{
        MirDeclaration, MirExpression, MirFunction, MirProgram, MirStatement, MirStructure, MirType,
    },
    parse, Identifier, StringLiteral,
};

use core::fmt::{Display, Error, Formatter};

#[derive(Clone, Debug)]
pub struct HirProgram(Vec<HirDeclaration>, i32);

impl HirProgram {
    pub fn new(decls: Vec<HirDeclaration>, heap_size: i32) -> Self {
        Self(decls, heap_size)
    }

    pub fn get_declarations(&self) -> &[HirDeclaration] {
        let Self(decls, _) = self;
        decls
    }

    pub fn get_heap_size(&self) -> i32 {
        let Self(_, heap_size) = self;
        *heap_size
    }

    fn set_heap_size(&mut self, size: i32) {
        self.1 = size;
    }

    pub fn compile(&self, cwd: &PathBuf) -> Result<MirProgram, HirError> {
        let mut constants = BTreeMap::new();
        let mut mir_decls = Vec::new();
        let mut heap_size = self.get_heap_size();
        for decl in self.get_declarations() {
            match decl {
                HirDeclaration::Constant(name, constant) => {
                    constants.insert(name.clone(), constant.clone());
                }
                HirDeclaration::Function(func) => {
                    mir_decls.push(MirDeclaration::Function(func.to_mir_fn(&constants)?))
                }
                HirDeclaration::Structure(structure) => mir_decls.push(MirDeclaration::Structure(
                    structure.to_mir_struct(&constants)?,
                )),
                HirDeclaration::Include(filename) => {
                    if let Ok(contents) = read_to_string(cwd.join(filename)) {
                        mir_decls.extend(parse(contents).compile(cwd)?.get_declarations());
                    } else {
                        eprintln!("error: could not include file '{}'", filename);
                        exit(1);
                    }
                }
                HirDeclaration::HeapSize(size) => {
                    heap_size = *size;
                }
            }
        }

        Ok(MirProgram::new(mir_decls, heap_size))
    }
}

#[derive(Clone, Debug)]
pub enum HirError {
    ConstantNotDefined(Identifier),
}

impl Display for HirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::ConstantNotDefined(name) => write!(f, "constant '{}' is not defined", name),
        }
    }
}

#[derive(Clone, Debug)]
pub enum HirType {
    Pointer(Box<Self>),
    Void,
    Float,
    Character,
    Structure(Identifier),
}

impl HirType {
    pub fn to_mir_type(&self) -> MirType {
        match self {
            Self::Pointer(inner) => inner.to_mir_type().refer(),
            Self::Void => MirType::void(),
            Self::Float => MirType::float(),
            Self::Character => MirType::character(),
            Self::Structure(name) => MirType::structure(name.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum HirDeclaration {
    Constant(Identifier, HirConstant),
    Function(HirFunction),
    Structure(HirStructure),
    Include(String),
    HeapSize(i32),
}

#[derive(Clone, Debug)]
pub struct HirStructure {
    name: Identifier,
    size: HirConstant,
    methods: Vec<HirFunction>,
}

impl HirStructure {
    pub fn new(name: Identifier, size: HirConstant, methods: Vec<HirFunction>) -> Self {
        Self {
            name,
            size,
            methods,
        }
    }

    pub fn to_mir_struct(
        &self,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirStructure, HirError> {
        let mut mir_methods = Vec::new();
        for method in self.methods.clone() {
            mir_methods.push(method.to_mir_fn(constants)?);
        }

        Ok(MirStructure::new(
            self.name.clone(),
            self.size.to_value(constants)? as i32,
            mir_methods,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct HirFunction {
    name: Identifier,
    args: Vec<(Identifier, HirType)>,
    return_type: HirType,
    body: Vec<HirStatement>,
}

impl HirFunction {
    pub fn new(
        name: Identifier,
        args: Vec<(Identifier, HirType)>,
        return_type: HirType,
        body: Vec<HirStatement>,
    ) -> Self {
        Self {
            name,
            args,
            return_type,
            body,
        }
    }

    pub fn to_mir_fn(
        &self,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirFunction, HirError> {
        let mut mir_args = Vec::new();
        for (arg_name, arg_type) in self.args.clone() {
            mir_args.push((arg_name.clone(), arg_type.to_mir_type()));
        }

        let mut mir_body = Vec::new();
        for stmt in self.body.clone() {
            mir_body.push(stmt.to_mir_stmt(constants)?);
        }

        Ok(MirFunction::new(
            self.name.clone(),
            mir_args,
            self.return_type.to_mir_type(),
            mir_body,
        ))
    }
}

#[derive(Clone, Debug)]
pub enum HirConstant {
    Float(f64),
    Character(char),

    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),

    Constant(Identifier),
}

impl HirConstant {
    pub fn to_value(&self, constants: &BTreeMap<Identifier, Self>) -> Result<f64, HirError> {
        Ok(match self {
            Self::Float(n) => *n,
            Self::Character(ch) => *ch as u8 as f64,

            Self::Add(l, r) => l.to_value(constants)? + r.to_value(constants)?,
            Self::Subtract(l, r) => l.to_value(constants)? - r.to_value(constants)?,
            Self::Multiply(l, r) => l.to_value(constants)? * r.to_value(constants)?,
            Self::Divide(l, r) => l.to_value(constants)? / r.to_value(constants)?,

            Self::Constant(name) => {
                if let Some(value) = constants.get(name) {
                    value.to_value(constants)?
                } else {
                    return Err(HirError::ConstantNotDefined(name.clone()));
                }
            }
        })
    }
}

#[derive(Clone, Debug)]
pub enum HirStatement {
    /// An HIR let expression with a manually assigned type
    Define(Identifier, HirType, HirExpression),
    /// An HIR let expression with an automatically assigned type
    AutoDefine(Identifier, HirExpression),
    /// A variable assignment
    AssignVariable(Identifier, HirExpression),
    /// An assignment to a dereferenced address
    AssignAddress(HirExpression, HirExpression),

    /// An HIR for loop
    For(Box<Self>, HirExpression, Box<Self>, Vec<Self>),
    /// An HIR while loop
    While(HirExpression, Vec<Self>),
    /// An HIR if statement
    If(HirExpression, Vec<Self>),
    /// An HIR if statement with an else clause
    IfElse(HirExpression, Vec<Self>, Vec<Self>),

    /// An HIR free statement to deallocate memory
    Free(HirExpression, HirExpression),
    /// Any expression
    Expression(HirExpression),
}

impl HirStatement {
    /// Lower an HIR statement into an equivalent MIR statement
    pub fn to_mir_stmt(
        &self,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirStatement, HirError> {
        Ok(match self {
            Self::Define(name, data_type, expr) => MirStatement::Define(
                name.clone(),
                data_type.to_mir_type(),
                expr.to_mir_expr(constants)?,
            ),
            Self::AutoDefine(name, expr) => {
                MirStatement::AutoDefine(name.clone(), expr.to_mir_expr(constants)?)
            }

            Self::AssignVariable(name, expr) => {
                MirStatement::AssignVariable(name.clone(), expr.to_mir_expr(constants)?)
            }
            Self::AssignAddress(addr, expr) => MirStatement::AssignAddress(
                addr.to_mir_expr(constants)?,
                expr.to_mir_expr(constants)?,
            ),

            Self::For(pre, cond, post, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants)?);
                }
                MirStatement::For(
                    Box::new(pre.to_mir_stmt(constants)?),
                    cond.to_mir_expr(constants)?,
                    Box::new(post.to_mir_stmt(constants)?),
                    mir_body,
                )
            }

            Self::While(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants)?);
                }
                MirStatement::While(cond.to_mir_expr(constants)?, mir_body)
            }

            Self::If(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants)?);
                }
                MirStatement::If(cond.to_mir_expr(constants)?, mir_body)
            }

            Self::IfElse(cond, then_body, else_body) => {
                let mut mir_then_body = Vec::new();
                for stmt in then_body {
                    mir_then_body.push(stmt.to_mir_stmt(constants)?);
                }
                let mut mir_else_body = Vec::new();
                for stmt in else_body {
                    mir_else_body.push(stmt.to_mir_stmt(constants)?);
                }
                MirStatement::IfElse(cond.to_mir_expr(constants)?, mir_then_body, mir_else_body)
            }

            Self::Free(addr, size) => {
                MirStatement::Free(addr.to_mir_expr(constants)?, size.to_mir_expr(constants)?)
            }

            Self::Expression(expr) => MirStatement::Expression(expr.to_mir_expr(constants)?),
        })
    }
}

#[derive(Clone, Debug)]
pub enum HirExpression {
    Constant(HirConstant),

    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),

    Greater(Box<Self>, Box<Self>),
    Less(Box<Self>, Box<Self>),
    GreaterEqual(Box<Self>, Box<Self>),
    LessEqual(Box<Self>, Box<Self>),

    Refer(Identifier),
    Deref(Box<Self>),

    Void,
    String(StringLiteral),
    Variable(Identifier),

    Alloc(Box<Self>),

    Call(Identifier, Vec<Self>),
    ForeignCall(Identifier, Vec<Self>),
    Method(Box<Self>, Identifier, Vec<Self>),
    Index(Box<Self>, Box<Self>),
}

impl HirExpression {
    pub fn to_mir_expr(
        &self,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirExpression, HirError> {
        Ok(match self {
            /// Convert a constant expression into a float literal
            Self::Constant(constant) => MirExpression::Float(constant.to_value(constants)?),

            Self::Add(l, r) => MirExpression::Add(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Greater(l, r) => MirExpression::Greater(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::GreaterEqual(l, r) => MirExpression::GreaterEqual(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Less(l, r) => MirExpression::Less(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::LessEqual(l, r) => MirExpression::LessEqual(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Subtract(l, r) => MirExpression::Subtract(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Multiply(l, r) => MirExpression::Multiply(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Divide(l, r) => MirExpression::Divide(
                Box::new(l.to_mir_expr(constants)?),
                Box::new(r.to_mir_expr(constants)?),
            ),

            Self::Refer(name) => MirExpression::Refer(name.clone()),
            Self::Deref(value) => MirExpression::Deref(Box::new(value.to_mir_expr(constants)?)),

            Self::Void => MirExpression::Void,
            Self::String(string) => MirExpression::String(string.clone()),

            /// If a variable is actually a constant,
            /// replace it with its constant value
            Self::Variable(name) => {
                if let Some(val) = constants.get(name) {
                    MirExpression::Float(val.to_value(constants)?)
                } else {
                    MirExpression::Variable(name.clone())
                }
            }

            Self::Alloc(value) => MirExpression::Alloc(Box::new(value.to_mir_expr(constants)?)),

            Self::Call(name, arguments) => MirExpression::Call(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(constants)?);
                }
                result
            }),

            Self::ForeignCall(name, arguments) => MirExpression::ForeignCall(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(constants)?);
                }
                result
            }),

            Self::Method(instance, name, arguments) => {
                MirExpression::Method(Box::new(instance.to_mir_expr(constants)?), name.clone(), {
                    let mut result = Vec::new();
                    for arg in arguments {
                        result.push(arg.to_mir_expr(constants)?);
                    }
                    result
                })
            }

            Self::Index(ptr, idx) => MirExpression::Index(
                Box::new(ptr.to_mir_expr(constants)?),
                Box::new(idx.to_mir_expr(constants)?),
            ),
        })
    }
}
