use std::collections::BTreeMap;

use crate::{
    mir::{
        MirDeclaration, MirExpression, MirFunction, MirProgram, MirStatement, MirStructure, MirType,
    },
    Identifier, StringLiteral,
};

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

    pub fn set_heap_size(&mut self, size: i32) {
        self.1 = size;
    }

    pub fn get_heap_size(&self) -> i32 {
        let Self(_, heap_size) = self;
        *heap_size
    }

    pub fn compile(&self) -> Result<MirProgram, HirError> {
        let mut constants = BTreeMap::new();
        let mut mir_decls = Vec::new();
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
            }
        }

        Ok(MirProgram::new(mir_decls, self.get_heap_size()))
    }
}

#[derive(Clone, Debug)]
pub enum HirError {
    ConstantNotDefined(Identifier),
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
    Void,
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
            Self::Void => 0.0,
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
    Define(Identifier, HirType, HirExpression),
    AssignVariable(Identifier, HirExpression),
    AssignAddress(HirExpression, HirExpression),

    For(Box<Self>, HirExpression, Box<Self>, Vec<Self>),
    While(HirExpression, Vec<Self>),
    If(HirExpression, Vec<Self>),

    Free(HirExpression, HirExpression),
    Expression(HirExpression),
}

impl HirStatement {
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

    Refer(Identifier),
    Deref(Box<Self>),

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
            Self::Constant(constant) => MirExpression::Float(constant.to_value(constants)?),

            Self::Add(l, r) => MirExpression::Add(
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

            Self::String(string) => MirExpression::String(string.clone()),
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
