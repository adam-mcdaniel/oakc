use std::{
    collections::BTreeMap,
    fmt::{Display, Error, Formatter},
    fs::read_to_string,
    path::PathBuf,
    process::exit,
};

use crate::{
    mir::{
        MirDeclaration, MirExpression, MirFunction, MirProgram, MirStatement, MirStructure, MirType,
    },
    parse, Identifier, StringLiteral,
};

#[derive(Clone, Debug)]
pub struct HirProgram(Vec<HirDeclaration>, i32);

impl HirProgram {
    pub const MINIMUM_MEMORY_SIZE: i32 = 128;

    pub fn new(decls: Vec<HirDeclaration>, memory_size: i32) -> Self {
        Self(decls, memory_size)
    }

    pub fn get_declarations(&self) -> &Vec<HirDeclaration> {
        let Self(decls, _) = self;
        decls
    }

    pub fn extend_declarations(&mut self, decls: &Vec<HirDeclaration>) {
        self.0.extend(decls.clone())
    }

    fn get_memory_size(&self) -> i32 {
        let Self(_, memory_size) = self;
        *memory_size
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

    pub fn generate_docs(
        &self,
        filename: String,
        constants: &mut BTreeMap<String, HirConstant>,
        ignore_header: bool,
    ) -> String {
        let mut header = String::new();
        if !ignore_header {
            header = format!("# {}\n", filename.trim())
        }

        let mut content = String::new();
        for decl in self.get_declarations() {
            match decl {
                HirDeclaration::DocumentHeader(s) => {
                    if !ignore_header {
                        header += s;
                        header += "\n";
                    }
                    continue;
                }
                HirDeclaration::Structure(structure) => content += &structure.generate_docs(),
                HirDeclaration::Function(function) => content += &function.generate_docs(false),
                HirDeclaration::Constant(doc, name, constant) => {
                    content += &format!("### *const* **{}** = {}\n---", name, constant);
                    if let Some(s) = doc {
                        content += "\n";
                        content += &s.trim();
                    }
                }
                _ => continue,
            }

            content += "\n";
        }
        header + &content
    }

    pub fn compile(
        &mut self,
        cwd: &PathBuf,
        constants: &mut BTreeMap<String, HirConstant>,
    ) -> Result<MirProgram, HirError> {
        let mut mir_decls = Vec::new();
        let mut memory_size = self.get_memory_size();
        let mut std_required = None;

        // Iterate over the declarations and retreive the constants
        for decl in self.get_declarations() {
            if let HirDeclaration::Constant(_, name, constant) = decl {
                constants.insert(name.clone(), constant.clone());
            }
        }

        for decl in self.get_declarations() {
            /// The reason we don't handle conditional compilation here
            /// is because it does not allow constants to be defined in
            /// the `if` statements' body.
            match decl {
                HirDeclaration::Function(func) => mir_decls.push(MirDeclaration::Function(
                    func.to_mir_fn(self.get_declarations(), &constants)?,
                )),
                HirDeclaration::Structure(structure) => mir_decls.push(MirDeclaration::Structure(
                    structure.to_mir_struct(self.get_declarations(), &constants)?,
                )),
                HirDeclaration::RequireStd => {
                    if let Some(false) = std_required {
                        return Err(HirError::ConflictingStdReqs);
                    } else {
                        std_required = Some(true)
                    }
                }
                HirDeclaration::NoStd => {
                    if let Some(true) = std_required {
                        return Err(HirError::ConflictingStdReqs);
                    } else {
                        std_required = Some(false)
                    }
                }
                HirDeclaration::Assert(constant) => {
                    if constant.to_value(self.get_declarations(), constants)? == 0.0 {
                        return Err(HirError::FailedAssertion(constant.clone()));
                    }
                }
                HirDeclaration::Extern(filename) => {
                    let file_path = cwd.join(filename.clone());
                    mir_decls.push(MirDeclaration::Extern(file_path))
                }
                HirDeclaration::Error(err) => return Err(HirError::UserError(err.clone())),

                HirDeclaration::Memory(size) => {
                    if *size >= Self::MINIMUM_MEMORY_SIZE {
                        memory_size = *size;
                    } else {
                        return Err(HirError::MemorySizeTooSmall(*size));
                    }
                }
                _ => {}
            }
        }

        Ok(MirProgram::new(mir_decls, memory_size))
    }
}

#[derive(Clone, Debug)]
pub enum HirError {
    /// There are some programs that can't possibly
    /// run on a minimum amount of memory. To reduce
    /// the incidence of this happening, we check against
    /// a minimum memory size.
    MemorySizeTooSmall(i32),
    /// If a constant is used without it being defined,
    /// then throw this error.
    ConstantNotDefined(Identifier),
    /// If BOTH the `std` and `no_std` flags are used
    /// in a program, then there are conflicting requirements
    /// for including the standard library. Throw this error
    /// if that is the case.
    ConflictingStdReqs,
    /// If a compile time assertion fails, throw an error
    FailedAssertion(HirConstant),
    /// This is a user defined error using the `error` flag
    UserError(String),
    /// This returns an error if a type is not defined. This was
    /// specifically implemented for defining the `sizeof` operator.
    TypeNotDefined(String),
    /// This occurs when a literal expression is cast as a pointer.
    /// This isn't ACTUALLY bad, but it's intended to promote type correctness.
    CastLiteralAsPointer(HirType),
}

impl Display for HirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::MemorySizeTooSmall(n) => write!(
                f,
                "specified stack + heap memory size '{}' is too small. use '{}' or greater",
                n,
                HirProgram::MINIMUM_MEMORY_SIZE
            ),
            Self::ConstantNotDefined(name) => write!(f, "constant '{}' is not defined", name),
            Self::UserError(err) => write!(f, "{}", err),
            Self::ConflictingStdReqs => {
                write!(f, "conflicting 'require_std' and 'no_std' flags present")
            }
            Self::FailedAssertion(assertion) => write!(f, "failed assertion '{}'", assertion),
            Self::TypeNotDefined(type_name) => write!(f, "type not defined '{}'", type_name),
            Self::CastLiteralAsPointer(t) => write!(f, "cannot cast literal to type '{}'", t),
        }
    }
}

/// This enum represents a type name in an expression.
/// Take for example the declaration `fn test(x: num) -> &void`.
/// `num` and `&void` are both `HirType` instances.
#[derive(Clone, Debug, PartialEq)]
pub enum HirType {
    /// A pointer to another type
    Pointer(Box<Self>),
    /// The unit type, or the type that represents no
    /// return value.
    Void,
    /// The floating point number type
    Float,
    /// The boolean type
    Boolean,
    /// The character type
    Character,
    /// A user defined type
    Structure(Identifier),
}

impl HirType {
    /// Get the size of the type on the stack.
    /// For primitive types, this is straight forward. For
    /// user types, though, we have to search for the structure
    /// in the list of declarations and find its type.
    pub fn get_size(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<i32, HirError> {
        Ok(match self {
            // A void type has size zero
            Self::Void => 0,
            // A pointer, a number, a boolean, and a character
            // all have a size of 1 on the stack
            Self::Pointer(_) | Self::Float | Self::Boolean | Self::Character => 1,
            Self::Structure(name) => {
                for decl in decls {
                    if let HirDeclaration::Structure(structure) = decl {
                        if name == structure.get_name() {
                            // Get the size of the structure with the type's name
                            return structure.get_size(decls, constants);
                        }
                    }
                }
                return Err(HirError::TypeNotDefined(name.clone()));
            }
        })
    }

    /// Is this type a pointer type?
    pub fn is_pointer(&self) -> bool {
        match self {
            Self::Pointer(_) => true,
            _ => false,
        }
    }

    /// Lower this type to the MIR type representation
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

impl Display for HirType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::Pointer(t) => write!(f, "&{}", t),
            Self::Void => write!(f, "{}", MirType::VOID),
            Self::Float => write!(f, "{}", MirType::FLOAT),
            Self::Boolean => write!(f, "{}", MirType::BOOLEAN),
            Self::Character => write!(f, "{}", MirType::CHAR),
            Self::Structure(name) => write!(f, "{}", name),
        }
    }
}

/// This type represents all the different flags
/// and definitions that the user has access to,
/// such as fn, const, and struct definitions,
/// and conditional compilation statements.
///
/// All constants are compiled to type float, even
/// characters and other literals.
#[derive(Clone, Debug)]
pub enum HirDeclaration {
    /// This adds a docstring to the head of the document
    /// that is displayed with the `doc` subcommand.
    DocumentHeader(String),
    /// Define a constant with an optional docstring.
    Constant(Option<String>, Identifier, HirConstant),
    /// Define a function
    Function(HirFunction),
    /// Define a structure
    Structure(HirStructure),
    /// Use the `assert` compiler flag
    Assert(HirConstant),
    /// Allow the user to throw their own custom errors
    Error(String),
    /// Include a foreign file using the `extern` flag.
    Extern(String),
    /// Set the memory used for the stack and heap.
    Memory(i32),
    /// Mark that the standard library is required for the program
    RequireStd,
    /// Mark that the standard library is not allowed for the program
    NoStd,
    /// Do nothing
    Pass,
}

/// This type represents a user defined structure.
#[derive(Clone, Debug)]
pub struct HirStructure {
    /// The optional docstring for the structure
    doc: Option<String>,
    /// The name of the structure
    name: Identifier,
    /// The size of the structure on the stack
    size: HirConstant,
    /// The list of methods for the structure.
    methods: Vec<HirFunction>,
    /// This represents whether or not the type is
    /// movable: if the type requires the copy and
    /// drop methods to be called. This allows
    /// users to write code with less restrictions
    /// for types that don't need to be dropped.
    is_movable: bool,
}

impl HirStructure {
    pub fn new(
        doc: Option<String>,
        name: Identifier,
        size: HirConstant,
        methods: Vec<HirFunction>,
        is_movable: bool,
    ) -> Self {
        Self {
            doc,
            name,
            size,
            methods,
            is_movable,
        }
    }

    /// Get the structure definition's name
    fn get_name(&self) -> &Identifier {
        &self.name
    }

    /// Get the size that the structure consumes on the stack
    fn get_size(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<i32, HirError> {
        // Convert the `size` constant into an integeral value
        self.size
            .to_value(decls, constants)
            .and_then(|n| Ok(n as i32))
    }

    /// Generate the documentation for the structure using the
    /// docstring and the docstrings of each method.
    fn generate_docs(&self) -> String {
        // Add a header for the output markdown
        let mut result = format!("## *type* **{}**\n", self.name);
        // If a docstring is defined, then
        // add it to the output
        if let Some(doc) = &self.doc {
            result += &(doc.trim().to_string() + "\n");
        }
        // Add documentation for each member function
        // as a method
        for method in &self.methods {
            result += &method.generate_docs(true)
        }
        result
    }

    /// Convert the HIR structure into its MIR
    /// structure representation.
    fn to_mir_struct(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirStructure, HirError> {
        // Convert each method into an MIR function
        let mut mir_methods = Vec::new();
        for method in &self.methods {
            mir_methods.push(method.to_mir_fn(decls, constants)?);
        }

        // Create an MIR structure with this structure's
        // name, size, methods, and movability.
        Ok(MirStructure::new(
            self.name.clone(),
            self.size.to_value(decls, constants)? as i32,
            mir_methods,
            self.is_movable,
        ))
    }
}

/// This type represents a user defined function.
#[derive(Clone, Debug, PartialEq)]
pub struct HirFunction {
    /// The optional docstring for the function
    doc: Option<String>,
    /// The name of the function
    name: Identifier,
    /// The parameters of the function
    args: Vec<(Identifier, HirType)>,
    /// The functions return type
    return_type: HirType,
    /// The body of the function
    body: Vec<HirStatement>,
}

impl HirFunction {
    pub fn new(
        doc: Option<String>,
        name: Identifier,
        args: Vec<(Identifier, HirType)>,
        return_type: HirType,
        body: Vec<HirStatement>,
    ) -> Self {
        Self {
            doc,
            name,
            args,
            return_type,
            body,
        }
    }

    /// Generate the documentation for the function.
    fn generate_docs(&self, is_method: bool) -> String {
        let mut result = if is_method {
            // If the function is a method, display the
            // function under a bullet point
            format!("* *fn* **{}**(", self.name)
        } else {
            // If the function is not a method, display
            // the function under its own header
            format!("### *fn* **{}**(", self.name)
        };

        // For each argument, display its name and type
        for (i, (arg_name, arg_type)) in self.args.iter().enumerate() {
            result += &format!("*{}*: {}, ", arg_name, arg_type)
        }
        // Remove the last space and comma from the last argument
        if !self.args.is_empty() {
            result.pop();
            result.pop();
        }

        // Add the close parantheses
        result += ")";

        if self.return_type != HirType::Void {
            // If the function is a non-void function, add the return type
            result += " *->* ";
            result += &self.return_type.to_string();
        }

        result += "\n";

        if let Some(doc) = &self.doc {
            result += if is_method { "  - " } else { "---\n" };
            result += &(doc.trim().to_string() + "\n");
        }
        result
    }

    /// Convert the HIR function into its MIR equivalent
    fn to_mir_fn(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirFunction, HirError> {
        // Convert each of the argument type to MIR types
        let mut mir_args = Vec::new();
        for (arg_name, arg_type) in self.args.clone() {
            mir_args.push((arg_name.clone(), arg_type.to_mir_type()));
        }

        // For each statement in the functions body,
        // convert it to an MIR statement.
        let mut mir_body = Vec::new();
        for stmt in self.body.clone() {
            mir_body.push(stmt.to_mir_stmt(decls, constants)?);
        }

        Ok(MirFunction::new(
            self.name.clone(),
            mir_args,
            self.return_type.to_mir_type(),
            mir_body,
        ))
    }
}

/// This type represents all constant expressions.
#[derive(Clone, Debug, PartialEq)]
pub enum HirConstant {
    /// A constant Float
    Float(f64),
    /// A constant Character
    Character(char),
    /// A constant Boolean
    True,
    False,

    /// Add two constants
    Add(Box<Self>, Box<Self>),
    /// Subtract two constants
    Subtract(Box<Self>, Box<Self>),
    /// Multiply two constants
    Multiply(Box<Self>, Box<Self>),
    /// Divide two constants
    Divide(Box<Self>, Box<Self>),

    /// Boolean And two constants
    And(Box<Self>, Box<Self>),
    /// Boolean Or two constants
    Or(Box<Self>, Box<Self>),
    /// Boolean Not a constant
    Not(Box<Self>),

    /// Compare two constants
    Greater(Box<Self>, Box<Self>),
    Less(Box<Self>, Box<Self>),
    GreaterEqual(Box<Self>, Box<Self>),
    LessEqual(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),

    /// A named constant
    Constant(Identifier),
    /// Determines whether a constant is defined
    IsDefined(String),
    /// The size of a constant
    SizeOf(HirType),
    /// A constant expression that is contingent on another constant expression
    Conditional(Box<Self>, Box<Self>, Box<Self>),
}

impl Display for HirConstant {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::Conditional(cond, then, otherwise) => {
                write!(f, "{} ? {} : {}", cond, then, otherwise)
            }
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
            Self::SizeOf(name) => write!(f, "sizeof(\"{}\")", name),
            Self::IsDefined(name) => write!(f, "is_defined(\"{}\")", name),
            Self::Not(expr) => write!(f, "!{}", expr),
        }
    }
}

impl HirConstant {
    pub fn boolean(b: bool) -> Self {
        if b {
            Self::True
        } else {
            Self::False
        }
    }

    /// Get the type of a constant value
    fn get_type(&self, constants: &BTreeMap<Identifier, Self>) -> Result<HirType, HirError> {
        Ok(match self {
            Self::Conditional(_, a, _)
            | Self::Add(a, _)
            | Self::Subtract(a, _)
            | Self::Multiply(a, _)
            | Self::Divide(a, _) => a.get_type(constants)?,

            Self::True
            | Self::False
            | Self::And(_, _)
            | Self::Or(_, _)
            | Self::Not(_)
            | Self::Greater(_, _)
            | Self::GreaterEqual(_, _)
            | Self::Less(_, _)
            | Self::LessEqual(_, _)
            | Self::Equal(_, _)
            | Self::NotEqual(_, _)
            | Self::IsDefined(_) => HirType::Boolean,

            Self::Constant(name) => {
                if let Some(value) = constants.get(name) {
                    value.get_type(constants)?
                } else {
                    return Err(HirError::ConstantNotDefined(name.clone()));
                }
            }

            Self::Character(_) => HirType::Character,

            Self::Float(_) | Self::SizeOf(_) => HirType::Float,
        })
    }

    /// Find a constants floating point value.
    pub fn to_value(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, Self>,
    ) -> Result<f64, HirError> {
        Ok(match self {
            Self::Conditional(cond, then, otherwise) => {
                if cond.to_value(decls, constants)? != 0.0 {
                    // If the constant condition is true, then use
                    // the first constant branch
                    then.to_value(decls, constants)?
                } else {
                    // If the constant condition is false, then use
                    // the second constant branch
                    otherwise.to_value(decls, constants)?
                }
            }

            Self::True => 1.0,
            Self::False => 0.0,

            Self::Float(n) => *n,
            Self::Character(ch) => *ch as u8 as f64,

            Self::And(l, r) => {
                if l.to_value(decls, constants)? != 0.0 && r.to_value(decls, constants)? != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Or(l, r) => {
                if l.to_value(decls, constants)? != 0.0 || r.to_value(decls, constants)? != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Equal(l, r) => {
                if l.to_value(decls, constants)? == r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::NotEqual(l, r) => {
                if l.to_value(decls, constants)? != r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Greater(l, r) => {
                if l.to_value(decls, constants)? > r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Less(l, r) => {
                if l.to_value(decls, constants)? < r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::GreaterEqual(l, r) => {
                if l.to_value(decls, constants)? >= r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }
            Self::LessEqual(l, r) => {
                if l.to_value(decls, constants)? <= r.to_value(decls, constants)? {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Add(l, r) => l.to_value(decls, constants)? + r.to_value(decls, constants)?,
            Self::Subtract(l, r) => l.to_value(decls, constants)? - r.to_value(decls, constants)?,
            Self::Multiply(l, r) => l.to_value(decls, constants)? * r.to_value(decls, constants)?,
            Self::Divide(l, r) => l.to_value(decls, constants)? / r.to_value(decls, constants)?,

            Self::Constant(name) => {
                if let Some(value) = constants.get(name) {
                    value.to_value(decls, constants)?
                } else {
                    return Err(HirError::ConstantNotDefined(name.clone()));
                }
            }

            Self::SizeOf(t) => t.get_size(decls, constants)? as f64,

            Self::IsDefined(name) => {
                if let Some(value) = constants.get(name) {
                    1.0
                } else {
                    0.0
                }
            }

            Self::Not(constant) => {
                if constant.to_value(decls, constants)? != 0.0 {
                    0.0
                } else {
                    1.0
                }
            }
        })
    }
}

/// This type represents a statement used in a function body.
/// This includes loops, conditional statements, assignments,
/// and void expressions.
#[derive(Clone, Debug, PartialEq)]
pub enum HirStatement {
    /// An HIR let expression with a manually assigned type
    Define(Identifier, HirType, HirExpression),
    /// An HIR let expression with type inference
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
    fn to_mir_stmt(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirStatement, HirError> {
        Ok(match self {
            Self::Define(name, data_type, expr) => MirStatement::Define(
                name.clone(),
                data_type.to_mir_type(),
                expr.to_mir_expr(decls, constants)?,
            ),
            Self::AutoDefine(name, expr) => {
                MirStatement::AutoDefine(name.clone(), expr.to_mir_expr(decls, constants)?)
            }

            Self::AssignVariable(name, expr) => {
                MirStatement::AssignVariable(name.clone(), expr.to_mir_expr(decls, constants)?)
            }
            Self::AssignAddress(addr, expr) => MirStatement::AssignAddress(
                addr.to_mir_expr(decls, constants)?,
                expr.to_mir_expr(decls, constants)?,
            ),

            Self::For(pre, cond, post, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(decls, constants)?);
                }
                MirStatement::For(
                    Box::new(pre.to_mir_stmt(decls, constants)?),
                    cond.to_mir_expr(decls, constants)?,
                    Box::new(post.to_mir_stmt(decls, constants)?),
                    mir_body,
                )
            }

            Self::While(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(decls, constants)?);
                }
                MirStatement::While(cond.to_mir_expr(decls, constants)?, mir_body)
            }

            Self::If(cond, body) => {
                let mut mir_body = Vec::new();
                for stmt in body {
                    mir_body.push(stmt.to_mir_stmt(decls, constants)?);
                }
                MirStatement::If(cond.to_mir_expr(decls, constants)?, mir_body)
            }

            Self::IfElse(cond, then_body, else_body) => {
                // Convert the `then` case to MIR
                let mut mir_then_body = Vec::new();
                for stmt in then_body {
                    mir_then_body.push(stmt.to_mir_stmt(decls, constants)?);
                }
                // Convert the `else` case to MIR
                let mut mir_else_body = Vec::new();
                for stmt in else_body {
                    mir_else_body.push(stmt.to_mir_stmt(decls, constants)?);
                }
                MirStatement::IfElse(
                    cond.to_mir_expr(decls, constants)?,
                    mir_then_body,
                    mir_else_body,
                )
            }

            Self::Return(exprs) => {
                let mut mir_exprs = Vec::new();
                for expr in exprs {
                    mir_exprs.push(expr.to_mir_expr(decls, constants)?)
                }
                MirStatement::Return(mir_exprs)
            }

            Self::Free(addr, size) => MirStatement::Free(
                addr.to_mir_expr(decls, constants)?,
                size.to_mir_expr(decls, constants)?,
            ),

            Self::Expression(expr) => MirStatement::Expression(expr.to_mir_expr(decls, constants)?),
        })
    }
}

/// This type represents an expression that is used as
/// a value in a statement or in another expression.
#[derive(Clone, Debug, PartialEq)]
pub enum HirExpression {
    /// The size of a type as a number
    SizeOf(HirType),
    /// A constant expression
    Constant(HirConstant),

    /// The addition of two expressions
    Add(Box<Self>, Box<Self>),
    /// The subtraction of two expressions
    Subtract(Box<Self>, Box<Self>),
    /// The multiplication of two expressions
    Multiply(Box<Self>, Box<Self>),
    /// The division of two expressions
    Divide(Box<Self>, Box<Self>),

    /// Boolean not of an expression
    Not(Box<Self>),
    /// Boolean and of two expressions
    And(Box<Self>, Box<Self>),
    /// Boolean or of two expressions
    Or(Box<Self>, Box<Self>),

    /// Compare two expressions with the `>` operator
    Greater(Box<Self>, Box<Self>),
    /// Compare two expressions with the `<` operator
    Less(Box<Self>, Box<Self>),
    /// Compare two expressions with the `>=` operator
    GreaterEqual(Box<Self>, Box<Self>),
    /// Compare two expressions with the `<=` operator
    LessEqual(Box<Self>, Box<Self>),
    /// Compare two expressions with the `==` operator
    Equal(Box<Self>, Box<Self>),
    /// Compare two expressions with the `!=` operator
    NotEqual(Box<Self>, Box<Self>),

    /// Get the address of a variable
    Refer(Identifier),
    /// Dereference a pointer variable
    Deref(Box<Self>),

    /// The Unit expression
    Void,
    /// Boolean True
    True,
    /// Boolean False
    False,
    /// A character literal. This is expressed as an expression
    /// instead of a constant because constants are all of type float.
    Character(char),
    /// A stack allocated character array literal
    String(StringLiteral),
    /// A variable expression
    Variable(Identifier),

    /// Cast an expression's type to another type.
    TypeCast(Box<Self>, HirType),
    /// Mark an expression as moved. This means that the
    /// inner expression will not be copied or dropped.
    Move(Box<Self>),
    /// The address of N number of free
    /// memory cells on the stack.
    Alloc(Box<Self>),

    /// A function call
    Call(Identifier, Vec<Self>),
    /// A foreign function call
    ForeignCall(Identifier, Vec<Self>),
    /// A method call on an object
    Method(Box<Self>, Identifier, Vec<Self>),
    /// An index of a pointer value
    Index(Box<Self>, Box<Self>),

    /// A conditional expression
    Conditional(Box<Self>, Box<Self>, Box<Self>),
}

impl HirExpression {
    fn is_literal(&self) -> bool {
        match self {
            Self::Void
            | Self::True
            | Self::False
            | Self::Character(_)
            | Self::String(_)
            | Self::Constant(_) => true,
            _ => false,
        }
    }

    fn to_mir_expr(
        &self,
        decls: &Vec<HirDeclaration>,
        constants: &BTreeMap<Identifier, HirConstant>,
    ) -> Result<MirExpression, HirError> {
        Ok(match self {
            Self::Move(expr) => MirExpression::Move(Box::new(expr.to_mir_expr(decls, constants)?)),
            /// Get the size of a type and replace this expression
            /// with its float value.
            Self::SizeOf(t) => MirExpression::Float(t.get_size(decls, constants)? as f64),

            /// Convert a constant expression into a float literal
            Self::Constant(constant) => {
                let val = constant.to_value(decls, constants)?;
                match constant.get_type(constants)? {
                    HirType::Boolean => {
                        if val != 0.0 {
                            MirExpression::True
                        } else {
                            MirExpression::False
                        }
                    }

                    HirType::Character => MirExpression::Character(val as u8 as char),
                    _ => MirExpression::Float(val),
                }
            }

            Self::Add(l, r) => MirExpression::Add(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::True => MirExpression::True,
            Self::False => MirExpression::False,

            Self::Not(expr) => MirExpression::Not(Box::new(expr.to_mir_expr(decls, constants)?)),
            Self::And(l, r) => MirExpression::And(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),
            Self::Or(l, r) => MirExpression::Or(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Greater(l, r) => MirExpression::Greater(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::GreaterEqual(l, r) => MirExpression::GreaterEqual(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Less(l, r) => MirExpression::Less(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::LessEqual(l, r) => MirExpression::LessEqual(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Equal(l, r) => MirExpression::Equal(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::NotEqual(l, r) => MirExpression::NotEqual(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Subtract(l, r) => MirExpression::Subtract(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Multiply(l, r) => MirExpression::Multiply(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Divide(l, r) => MirExpression::Divide(
                Box::new(l.to_mir_expr(decls, constants)?),
                Box::new(r.to_mir_expr(decls, constants)?),
            ),

            Self::Refer(var_name) => MirExpression::Refer(var_name.clone()),
            Self::Deref(value) => {
                MirExpression::Deref(Box::new(value.to_mir_expr(decls, constants)?))
            }

            Self::Void => MirExpression::Void,
            Self::Character(ch) => MirExpression::Character(*ch),
            Self::String(string) => MirExpression::String(string.clone()),

            /// If a variable is actually a constant,
            /// replace it with its constant value
            Self::Variable(name) => {
                if let Some(val) = constants.get(name) {
                    HirExpression::Constant(val.clone()).to_mir_expr(decls, constants)?
                } else {
                    MirExpression::Variable(name.clone())
                }
            }

            Self::Alloc(value) => {
                MirExpression::Alloc(Box::new(value.to_mir_expr(decls, constants)?))
            }

            Self::TypeCast(expr, t) if expr.is_literal() && t.is_pointer() => {
                return Err(HirError::CastLiteralAsPointer(t.clone()))
            }

            Self::TypeCast(expr, t) => MirExpression::TypeCast(
                Box::new(expr.to_mir_expr(decls, constants)?),
                t.to_mir_type(),
            ),

            Self::Call(name, arguments) => MirExpression::Call(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(decls, constants)?);
                }
                result
            }),

            Self::ForeignCall(name, arguments) => MirExpression::ForeignCall(name.clone(), {
                let mut result = Vec::new();
                for arg in arguments {
                    result.push(arg.to_mir_expr(decls, constants)?);
                }
                result
            }),

            Self::Method(instance, name, arguments) => MirExpression::Method(
                Box::new(instance.to_mir_expr(decls, constants)?),
                name.clone(),
                {
                    let mut result = Vec::new();
                    for arg in arguments {
                        result.push(arg.to_mir_expr(decls, constants)?);
                    }
                    result
                },
            ),

            Self::Index(ptr, idx) => MirExpression::Index(
                Box::new(ptr.to_mir_expr(decls, constants)?),
                Box::new(idx.to_mir_expr(decls, constants)?),
            ),

            Self::Conditional(cond, then, otherwise) => MirExpression::Conditional(
                Box::new(cond.to_mir_expr(decls, constants)?),
                Box::new(then.to_mir_expr(decls, constants)?),
                Box::new(otherwise.to_mir_expr(decls, constants)?),
            ),
        })
    }
}
