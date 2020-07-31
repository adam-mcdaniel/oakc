use std::{collections::BTreeMap, fs::read_to_string, path::PathBuf, process::exit};

use crate::{
    mir::{
        MirDeclaration, MirExpression, MirFunction, MirProgram, MirStatement, MirStructure, MirType,
    },
    parse, Identifier, StringLiteral, Target,
};

use core::fmt::{Display, Error, Formatter};

#[derive(Clone, Debug)]
pub struct HirProgram(Vec<HirDeclaration>, i32);

impl HirProgram {
    pub fn new(decls: Vec<HirDeclaration>, heap_size: i32) -> Self {
        Self(decls, heap_size)
    }

    pub fn get_declarations(&self) -> &Vec<HirDeclaration> {
        let Self(decls, _) = self;
        decls
    }

    pub fn extend_declarations(&mut self, decls: &Vec<HirDeclaration>) {
        self.0.extend(decls.clone())
    }

    pub fn get_heap_size(&self) -> i32 {
        let Self(_, heap_size) = self;
        *heap_size
    }

    fn set_heap_size(&mut self, size: i32) {
        self.1 = size;
    }

    pub fn use_std(&self) -> bool {
        for decl in self.get_declarations() {
            match decl {
                HirDeclaration::NoStd => return false,
                HirDeclaration::RequireStd => return true,
                _ => {}
            }
        }
        false
    }

    pub fn compile(
        &mut self,
        cwd: &PathBuf,
        target: &impl Target,
        constants: &mut BTreeMap<String, HirConstant>,
    ) -> Result<MirProgram, HirError> {
        let mut mir_decls = Vec::new();
        let mut heap_size = self.get_heap_size();
        let mut std_required = None;

        // Iterate over the declarations and retreive the constants
        for decl in self.get_declarations() {
            if let HirDeclaration::Constant(name, constant) = decl {
                constants.insert(name.clone(), constant.clone());
            }
        }

        for decl in self.get_declarations() {
            /// The reason we don't handle conditional compilation here
            /// is because it does not allow constants to be defined in
            /// the `if` statements' body.
            match decl {
                HirDeclaration::Function(func) => mir_decls.push(MirDeclaration::Function(
                    func.to_mir_fn(&constants, target)?,
                )),
                HirDeclaration::Structure(structure) => mir_decls.push(MirDeclaration::Structure(
                    structure.to_mir_struct(&constants, target)?,
                )),
                HirDeclaration::RequireStd => if let Some(false) = std_required {
                    return Err(HirError::ConflictingStdReqs)
                } else {
                    std_required = Some(true)
                },
                HirDeclaration::NoStd => if let Some(true) = std_required {
                    return Err(HirError::ConflictingStdReqs)
                } else {
                    std_required = Some(false)
                },
                HirDeclaration::Assert(constant) => {
                    if constant.to_value(constants, target)? == 0.0 {
                        return Err(HirError::FailedAssertion(constant.clone()))
                    }
                }
                HirDeclaration::Extern(filename) => {
                    let file_path = cwd.join(filename.clone());
                    mir_decls.push(MirDeclaration::Extern(file_path))
                }
                HirDeclaration::Error(err) => return Err(HirError::UserError(err.clone())),
                HirDeclaration::Include(filename) => {
                    // This takes the path of the file in the `include` flag
                    // and appends it to the directory of the file which is
                    // including it.
                    //
                    // So, if `src/main.ok` includes "lib/all.ok",
                    // `file_path` will be equal to "src/lib/all.ok"
                    let file_path = cwd.join(filename.clone());
                    if let Ok(contents) = read_to_string(file_path.clone()) {
                        // Get the directory of the included file.

                        // If `src/main.ok` includes "lib/all.ok",
                        // `include_path` will be equal to "src/lib/"
                        let include_path = if let Some(dir) = file_path.parent() {
                            PathBuf::from(dir)
                        } else {
                            PathBuf::from("./")
                        };

                        // Compile the included file using the `include_path` as
                        // the current working directory.
                        mir_decls.extend(
                            parse(contents)
                                .compile(&include_path, target, constants)?
                                .get_declarations(),
                        );
                    } else {
                        eprintln!("error: could not include file '{}'", filename);
                        exit(1);
                    }
                }

                HirDeclaration::If(cond, code) => {
                    if cond.to_value(constants, target)? != 0.0 {
                        mir_decls.extend(
                            code.clone()
                                .compile(cwd, target, constants)?
                                .get_declarations(),
                        );
                    }
                }

                HirDeclaration::IfElse(cond, then_code, else_code) => {
                    if cond.to_value(constants, target)? != 0.0 {
                        mir_decls.extend(
                            then_code
                                .clone()
                                .compile(cwd, target, constants)?
                                .get_declarations(),
                        );
                    } else {
                        mir_decls.extend(
                            else_code
                                .clone()
                                .compile(cwd, target, constants)?
                                .get_declarations(),
                        );
                    }
                }

                HirDeclaration::HeapSize(size) => {
                    heap_size = *size;
                }
                _ => {}
            }
        }

        Ok(MirProgram::new(mir_decls, heap_size))
    }
}

#[derive(Clone, Debug)]
pub enum HirError {
    ConstantNotDefined(Identifier),
    ConflictingStdReqs,
    FailedAssertion(HirConstant),
    UserError(String),
}

impl Display for HirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::ConstantNotDefined(name) => write!(f, "constant '{}' is not defined", name),
            Self::UserError(err) => write!(f, "{}", err),
            Self::ConflictingStdReqs => write!(f, "conflicting 'require_std' and 'no_std' flags present"),
            Self::FailedAssertion(assertion) => write!(f, "failed assertion '{}'", assertion),
        }
    }
}

#[derive(Clone, Debug)]
pub enum HirType {
    Pointer(Box<Self>),
    Void,
    Float,
    Boolean,
    Character,
    Structure(Identifier),
}

impl HirType {
    pub fn to_mir_type(&self) -> MirType {
        match self {
            Self::Pointer(inner) => inner.to_mir_type().refer(),
            Self::Void => MirType::void(),
            Self::Float => MirType::float(),
            Self::Boolean => MirType::boolean(),
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
    Assert(HirConstant),
    If(HirConstant, HirProgram),
    IfElse(HirConstant, HirProgram, HirProgram),
    Error(String),
    Extern(String),
    Include(String),
    HeapSize(i32),
    RequireStd,
    NoStd
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
        target: &impl Target,
    ) -> Result<MirStructure, HirError> {
        let mut mir_methods = Vec::new();
        for method in self.methods.clone() {
            mir_methods.push(method.to_mir_fn(constants, target)?);
        }

        Ok(MirStructure::new(
            self.name.clone(),
            self.size.to_value(constants, target)? as i32,
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
        target: &impl Target,
    ) -> Result<MirFunction, HirError> {
        let mut mir_args = Vec::new();
        for (arg_name, arg_type) in self.args.clone() {
            mir_args.push((arg_name.clone(), arg_type.to_mir_type()));
        }

        let mut mir_body = Vec::new();
        for stmt in self.body.clone() {
            mir_body.push(stmt.to_mir_stmt(constants, target)?);
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
    Not(Box<Self>),
}


impl Display for HirConstant {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Float(n) => write!(f, "{}", n),
            Self::Character(ch) => write!(f, "'{}'", ch),
            Self::Add(l, r) => write!(f, "{}+{}", l, r),
            Self::Subtract(l, r) => write!(f, "{}-{}", l, r),
            Self::Multiply(l, r) => write!(f, "{}*{}", l, r),
            Self::Divide(l, r) => write!(f, "{}/{}", l, r),
            Self::And(l, r) => write!(f, "{}&&{}", l, r),
            Self::Or(l, r) => write!(f, "{}||{}", l, r),
            Self::Greater(l, r) => write!(f, "{}>{}", l, r),
            Self::Less(l, r) => write!(f, "{}<{}", l, r),
            Self::GreaterEqual(l, r) => write!(f, "{}>={}", l, r),
            Self::LessEqual(l, r) => write!(f, "{}<={}", l, r),
            Self::Equal(l, r) => write!(f, "{}=={}", l, r),
            Self::NotEqual(l, r) => write!(f, "{}!={}", l, r),
            Self::Constant(name) => write!(f, "{}", name),
            Self::IsDefined(name) => write!(f, "isdef(\"{}\")", name),
            Self::Not(expr) => write!(f, "!{}", expr),
        }
    }
}

impl HirConstant {
    pub fn to_value(
        &self,
        constants: &BTreeMap<Identifier, Self>,
        target: &impl Target,
    ) -> Result<f64, HirError> {
        Ok(match self {
            Self::True => 1.0,
            Self::False => 0.0,
            
            Self::Float(n) => *n,
            Self::Character(ch) => *ch as u8 as f64,

            Self::And(l, r) => {
                if l.to_value(constants, target)? != 0.0 && r.to_value(constants, target)? != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Or(l, r) => {
                if l.to_value(constants, target)? != 0.0 || r.to_value(constants, target)? != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Equal(l, r) => {
                if l.to_value(constants, target)? == r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::NotEqual(l, r) => {
                if l.to_value(constants, target)? != r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Greater(l, r) => {
                if l.to_value(constants, target)? > r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Less(l, r) => {
                if l.to_value(constants, target)? < r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::GreaterEqual(l, r) => {
                if l.to_value(constants, target)? >= r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::LessEqual(l, r) => {
                if l.to_value(constants, target)? <= r.to_value(constants, target)? {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Add(l, r) => l.to_value(constants, target)? + r.to_value(constants, target)?,
            Self::Subtract(l, r) => {
                l.to_value(constants, target)? - r.to_value(constants, target)?
            }
            Self::Multiply(l, r) => {
                l.to_value(constants, target)? * r.to_value(constants, target)?
            }
            Self::Divide(l, r) => l.to_value(constants, target)? / r.to_value(constants, target)?,

            Self::Constant(name) => match name.as_str() {
                "TARGET" => target.get_name() as u8 as f64,

                _ => {
                    if let Some(value) = constants.get(name) {
                        value.to_value(constants, target)?
                    } else {
                        return Err(HirError::ConstantNotDefined(name.clone()));
                    }
                }
            },

            Self::IsDefined(name) => {
                if let Some(value) = constants.get(name) {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Not(constant) => {
                if constant.to_value(constants, target)? != 0.0 {
                    0.0
                } else {
                    1.0
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
    /// Return one or more values at the end of a function
    Return(Vec<HirExpression>),

    /// Any expression
    Expression(HirExpression),
}

impl HirStatement {
    /// Lower an HIR statement into an equivalent MIR statement
    pub fn to_mir_stmt(
        &self,
        constants: &BTreeMap<Identifier, HirConstant>,
        target: &impl Target,
    ) -> Result<MirStatement, HirError> {
        Ok(match self {
            Self::Define(name, data_type, expr) => MirStatement::Define(
                name.clone(),
                data_type.to_mir_type(),
                expr.to_mir_expr(constants, target)?,
            ),
            Self::AutoDefine(name, expr) => {
                MirStatement::AutoDefine(name.clone(), expr.to_mir_expr(constants, target)?)
            }

            Self::AssignVariable(name, expr) => {
                MirStatement::AssignVariable(name.clone(), expr.to_mir_expr(constants, target)?)
            }
            Self::AssignAddress(addr, expr) => MirStatement::AssignAddress(
                addr.to_mir_expr(constants, target)?,
                expr.to_mir_expr(constants, target)?,
            ),

            Self::For(pre, cond, post, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants, target)?);
                }
                MirStatement::For(
                    Box::new(pre.to_mir_stmt(constants, target)?),
                    cond.to_mir_expr(constants, target)?,
                    Box::new(post.to_mir_stmt(constants, target)?),
                    mir_body,
                )
            }

            Self::While(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants, target)?);
                }
                MirStatement::While(cond.to_mir_expr(constants, target)?, mir_body)
            }

            Self::If(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(constants, target)?);
                }
                MirStatement::If(cond.to_mir_expr(constants, target)?, mir_body)
            }

            Self::IfElse(cond, then_body, else_body) => {
                let mut mir_then_body = Vec::new();
                for stmt in then_body {
                    mir_then_body.push(stmt.to_mir_stmt(constants, target)?);
                }
                let mut mir_else_body = Vec::new();
                for stmt in else_body {
                    mir_else_body.push(stmt.to_mir_stmt(constants, target)?);
                }
                MirStatement::IfElse(
                    cond.to_mir_expr(constants, target)?,
                    mir_then_body,
                    mir_else_body,
                )
            }

            Self::Return(exprs) => {
                let mut mir_exprs = Vec::new();
                for expr in exprs {
                    mir_exprs.push(expr.to_mir_expr(constants, target)?)
                }
                MirStatement::Return(mir_exprs)
            }

            Self::Free(addr, size) => MirStatement::Free(
                addr.to_mir_expr(constants, target)?,
                size.to_mir_expr(constants, target)?,
            ),

            Self::Expression(expr) => {
                MirStatement::Expression(expr.to_mir_expr(constants, target)?)
            }
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

    TypeCast(Box<Self>, HirType),
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
        target: &impl Target,
    ) -> Result<MirExpression, HirError> {
        Ok(match self {
            /// Convert a constant expression into a float literal
            Self::Constant(constant) => MirExpression::Float(constant.to_value(constants, target)?),

            Self::Add(l, r) => MirExpression::Add(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::True => MirExpression::True,
            Self::False => MirExpression::False,

            Self::Not(expr) => MirExpression::Not(
                Box::new(expr.to_mir_expr(constants, target)?),
            ),
            Self::And(l, r) => MirExpression::And(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),
            Self::Or(l, r) => MirExpression::Or(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Greater(l, r) => MirExpression::Greater(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::GreaterEqual(l, r) => MirExpression::GreaterEqual(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Less(l, r) => MirExpression::Less(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::LessEqual(l, r) => MirExpression::LessEqual(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Equal(l, r) => MirExpression::Equal(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::NotEqual(l, r) => MirExpression::NotEqual(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Subtract(l, r) => MirExpression::Subtract(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Multiply(l, r) => MirExpression::Multiply(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Divide(l, r) => MirExpression::Divide(
                Box::new(l.to_mir_expr(constants, target)?),
                Box::new(r.to_mir_expr(constants, target)?),
            ),

            Self::Refer(name) => MirExpression::Refer(name.clone()),
            Self::Deref(value) => {
                MirExpression::Deref(Box::new(value.to_mir_expr(constants, target)?))
            }

            Self::Void => MirExpression::Void,
            Self::Character(ch) => MirExpression::Character(*ch),
            Self::String(string) => MirExpression::String(string.clone()),

            /// If a variable is actually a constant,
            /// replace it with its constant value
            Self::Variable(name) => {
                if let Some(val) = constants.get(name) {
                    MirExpression::Float(val.to_value(constants, target)?)
                } else {
                    MirExpression::Variable(name.clone())
                }
            }

            Self::Alloc(value) => {
                MirExpression::Alloc(Box::new(value.to_mir_expr(constants, target)?))
            }

            Self::TypeCast(expr, t) => MirExpression::TypeCast(
                Box::new(expr.to_mir_expr(constants, target)?),
                t.to_mir_type(),
            ),

            Self::Call(name, arguments) => MirExpression::Call(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(constants, target)?);
                }
                result
            }),

            Self::ForeignCall(name, arguments) => MirExpression::ForeignCall(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(constants, target)?);
                }
                result
            }),

            Self::Method(instance, name, arguments) => MirExpression::Method(
                Box::new(instance.to_mir_expr(constants, target)?),
                name.clone(),
                {
                    let mut result = Vec::new();
                    for arg in arguments {
                        result.push(arg.to_mir_expr(constants, target)?);
                    }
                    result
                },
            ),

            Self::Index(ptr, idx) => MirExpression::Index(
                Box::new(ptr.to_mir_expr(constants, target)?),
                Box::new(idx.to_mir_expr(constants, target)?),
            ),
        })
    }
}
