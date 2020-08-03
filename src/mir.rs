use std::{
    collections::BTreeMap,
    fmt::{Display, Error, Formatter},
    path::PathBuf,
};

use crate::{
    asm::{AsmExpression, AsmFunction, AsmProgram, AsmStatement, AsmType},
    Identifier, StringLiteral,
};

/// A value representing an error while assembling the MIR code
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirError {
    /// Calling a function without defining it
    FunctionNotDefined(Identifier),
    /// Defining a type multiple times
    StructureRedefined(Identifier),
    /// Defining a structure with the name of a primitive type
    PrimitiveTypeRedefined(Identifier),
    /// Defining a function multiple times
    FunctionRedefined(Identifier),
    /// Using a variable without defining it
    VariableNotDefined(Identifier),
    /// Calling a method for a type where it is not defined
    MethodNotDefined(MirType, Identifier),
    /// Using a structure name as a type without defining it
    /// If this were acceptable, the compiler would never know
    /// the size of the variable.
    StructureNotDefined(Identifier),
    /// Dereferencing a non-pointer value
    DereferenceNonPointer(MirType),
    /// Indexing a void pointer
    /// This is inherently bad because void pointers have size
    /// zero. Indexing them is the same as dereferencing, but
    /// less efficient.
    IndexVoidPointer(MirExpression),
    /// Auto define a void pointer
    /// This is less of a type error itself and more of a safety net.
    /// Variables that hold the result of `alloc` must be the proper
    /// type for expressions like `ptr[n]` to work.
    AutoDefineVoidPointer(String, MirExpression),
    /// Mismatched types in a `let` statement
    DefineMismatchedType(String),
    /// Mismatched types in an assignment statement
    AssignMismatchedType(MirExpression),
    /// Arguments to a function call do not match parameter types
    ArgumentMismatchedType(MirExpression),
    /// Use a `free` statement using an address argument
    /// of a non-pointer type
    FreeNonPointer(MirExpression),
    /// Using a non-boolean expression for an if statement, and if-else
    /// statement, a while loop, or a for loop
    NonBooleanCondition(MirExpression),
    /// Using a non-number for an `alloc` call
    NonNumberAllocate(MirExpression),
    /// Indexing an array with a non-number value
    NonNumberIndex(MirExpression),
    /// Adding, subtracting, multiplying, or dividing two
    /// values where one or more of them is not a number.
    NonNumberBinaryOperation(MirExpression, MirExpression),
    /// Using the not operator or other unary operator
    /// on a non-number value.
    NonNumberUnaryOperation(MirExpression),
    /// Calling a function without enough arguments
    NotEnoughArguments(MirExpression),
    /// Calling a function with too many arguments
    TooManyArguments(MirExpression),
    /// Calling an associated function, such as a constructor,
    /// as a method
    CalledFunctionAsMethod(String),
    /// The return type of the function does not match the result
    /// of the function
    MismatchedReturnType(String),
    /// A function attempts to use multiple return statements
    MultipleReturns(String),
    /// An expression with non-void type is pushed onto the stack
    /// without being used by another expression or statement.
    NonVoidExpressionNotUsed(MirExpression),
    /// A bad typecast due to mismatched sizes in types. For example,
    /// a value with size `3` cannot be cast to a number with size `1`
    MismatchedCastSize(MirExpression, MirType),
    /// Attempted to use a return statement in a conditional expression,
    /// such as a while loop or if statement
    ConditionalReturn(String),
    /// A non-void function never returns
    NonVoidNoReturn(String),
}

/// Print an MIR error on the command line
impl Display for MirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::FunctionNotDefined(name) => write!(f, "function '{}' is not defined", name),
            Self::FunctionRedefined(name) => {
                write!(f, "function '{}' is defined multiple times", name)
            }
            Self::StructureNotDefined(name) => write!(f, "type '{}' is not defined", name),
            Self::StructureRedefined(name) => {
                write!(f, "type '{}' is defined multiple times", name)
            }
            Self::PrimitiveTypeRedefined(name) => {
                write!(f, "attempted to define structure with the primitive type name '{}'", name)
            }
            Self::VariableNotDefined(name) => write!(f, "variable '{}' is not defined", name),
            Self::MethodNotDefined(t, name) => {
                write!(f, "method '{}' is not defined for type '{}'", name, t)
            }
            Self::DereferenceNonPointer(t) => write!(f, "cannot dereference type '{}'", t),
            Self::IndexVoidPointer(expr) => write!(f, "cannot index void pointer '{}'", expr),
            Self::AutoDefineVoidPointer(var_name, expr) => write!(
                f,
                "used type inference when defining '{}' with a void pointer expression '{}'",
                var_name, expr
            ),

            Self::DefineMismatchedType(var_name) => write!(
                f,
                "mismatched types in 'let' statement when defining variable '{}'",
                var_name
            ),

            Self::AssignMismatchedType(lhs_expr) => {
                write!(f, "mismatched types when assigning to '{}'", lhs_expr)
            }
            Self::FreeNonPointer(address_expr) => {
                write!(f, "cannot free non-pointer '{}'", address_expr)
            }
            Self::NonBooleanCondition(cond_expr) => {
                write!(f, "cannot use non-boolean expression '{}' as a condition. try using the comparison operators, like '!=' or '=='", cond_expr)
            }
            Self::NonNumberAllocate(size_expr) => write!(
                f,
                "cannot use '{}' as a size argument in 'alloc' function",
                size_expr
            ),
            Self::NonNumberIndex(idx_expr) => write!(
                f,
                "cannot use non-number '{}' as an index for an array",
                idx_expr
            ),
            Self::NonNumberBinaryOperation(lhs, rhs) => write!(
                f,
                "cannot use non-numbers '{}' and '{}' in binary operation",
                lhs, rhs
            ),
            Self::NonNumberUnaryOperation(expr) => write!(
                f,
                "cannot use non-number '{}' in unary operation",
                expr
            ),
            Self::NotEnoughArguments(call_expr) => {
                write!(f, "too few arguments in function call '{}'", call_expr)
            }
            Self::TooManyArguments(call_expr) => {
                write!(f, "too many arguments in function call '{}'", call_expr)
            }
            Self::ArgumentMismatchedType(call_expr) => {
                write!(f, "mismatched types in function call '{}'", call_expr)
            }
            Self::CalledFunctionAsMethod(fn_name) => {
                write!(f, "called function '{}' as a method", fn_name)
            }
            Self::MismatchedReturnType(fn_name) => write!(
                f,
                "the return type of the function '{}' does not match the function's return value",
                fn_name
            ),
            Self::MultipleReturns(fn_name) => write!(
                f,
                "the function '{}' uses multiple return statements",
                fn_name
            ),
            Self::NonVoidExpressionNotUsed(expr) => write!(
                f,
                "the non-void expression '{}' is used but not consumed by another expression or statement",
                expr
            ),
            Self::MismatchedCastSize(expr, t) => write!(
                f,
                "cannot cast expression '{}' to type '{}' due to mismatched sizes",
                expr, t
            ),
            Self::ConditionalReturn(fn_name) => write!(
                f,
                "used a return statement within a conditional statement in the function '{}'",
                fn_name
            ),
            Self::NonVoidNoReturn(fn_name) => write!(
                f,
                "the non-void function '{}' never returns an expression",
                fn_name
            ),
        }
    }
}

#[derive(Clone, Debug, PartialOrd)]
pub struct MirType {
    /// The name of the type
    name: Identifier,
    /// How many references deep this type is,
    /// or how many `&` are in front of the type.
    ptr_level: i32,
}

impl MirType {
    /// The name of the float type in Oak code
    const FLOAT: &'static str = "num";
    /// The name of the character type in the Oak code
    const CHAR: &'static str = "char";
    /// The name of the unit type in the Oak code
    const VOID: &'static str = "void";
    /// The name of the bool type in Oak code
    const BOOLEAN: &'static str = "bool";

    /// A user defined type
    pub fn structure(name: Identifier) -> Self {
        Self { name, ptr_level: 0 }
    }

    /// Oak's boolean type
    pub fn boolean() -> Self {
        Self::structure(Identifier::from(Self::BOOLEAN))
    }

    /// Oak's floating-point type
    pub fn float() -> Self {
        Self::structure(Identifier::from(Self::FLOAT))
    }

    /// Oak's character type
    pub fn character() -> Self {
        Self::structure(Identifier::from(Self::CHAR))
    }

    /// Oak's unit type
    pub fn void() -> Self {
        Self::structure(Identifier::from(Self::VOID))
    }

    /// Is this type a pointer?
    pub fn is_pointer(&self) -> bool {
        self.ptr_level > 0
    }

    /// Lower this type into the ASM's representation of MIR types
    pub fn to_asm_type(
        &self,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<AsmType, MirError> {
        // Get the size of the underlying type with all references removed
        let mut result = AsmType::new(self.get_inner_size(structs)?);
        // Add the references to the type
        for _ in 0..self.ptr_level {
            result = result.refer();
        }
        Ok(result)
    }

    /// Get the size of this type on the stack
    fn get_size(&self, structs: &BTreeMap<Identifier, MirStructure>) -> Result<i32, MirError> {
        if self.is_pointer() {
            Ok(1)
        } else {
            self.get_inner_size(structs)
        }
    }

    /// Get the size of the underlying type with
    /// all references removed
    fn get_inner_size(
        &self,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<i32, MirError> {
        Ok(match self.name.as_str() {
            Self::VOID => 0,
            Self::BOOLEAN | Self::FLOAT | Self::CHAR => 1,
            other => {
                if let Some(structure) = structs.get(other) {
                    structure.get_size()
                } else {
                    return Err(MirError::StructureNotDefined(self.name.clone()));
                }
            }
        })
    }

    pub fn refer(&self) -> Self {
        let mut result = self.clone();
        result.ptr_level += 1;
        result
    }

    pub fn deref(&self) -> Result<Self, MirError> {
        if self.ptr_level > 0 {
            let mut result = self.clone();
            result.ptr_level -= 1;
            Ok(result)
        } else {
            Err(MirError::DereferenceNonPointer(self.clone()))
        }
    }

    fn is_void_ptr(&self) -> bool {
        self.name == Self::VOID && self.ptr_level == 1
    }

    fn method_to_function_name(&self, method_name: &Identifier) -> Identifier {
        format!("{}::{}", self.name, method_name)
    }
}

/// This implementation solely governs the rules for type-checking.
impl PartialEq for MirType {
    fn eq(&self, other: &Self) -> bool {
        // If two types are EXACTLY identical, they are equal
        if self.name == other.name && self.ptr_level == other.ptr_level {
            true
        } else {
            // (&void == &T) AND (&T == &void)
            (self.ptr_level == 1 && self.name == "void" && other.ptr_level == 1)
                || (other.ptr_level == 1 && other.name == "void" && self.ptr_level == 1)
        }
    }
}

impl Display for MirType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for _ in 0..self.ptr_level {
            write!(f, "&")?;
        }
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct MirProgram(Vec<MirDeclaration>, i32);

impl MirProgram {
    pub fn new(decls: Vec<MirDeclaration>, memory_size: i32) -> Self {
        Self(decls, memory_size)
    }

    pub fn get_declarations(&self) -> Vec<MirDeclaration> {
        (self.0).clone()
    }

    pub fn get_memory_size(&self) -> i32 {
        self.1
    }

    pub fn assemble(&self) -> Result<AsmProgram, MirError> {
        let Self(decls, memory_size) = self.clone();
        let mut externs = Vec::new();
        let mut funcs = BTreeMap::new();
        let mut structs = BTreeMap::new();
        let mut result = Vec::new();
        for decl in &decls {
            match decl {
                MirDeclaration::Function(func) => {
                    let name = func.get_name();
                    if funcs.contains_key(&name) {
                        return Err(MirError::FunctionRedefined(name));
                    } else {
                        funcs.insert(name, func.clone());
                    }
                }
                MirDeclaration::Structure(structure) => {
                    let name = structure.get_name();
                    if structs.contains_key(&name) {
                        return Err(MirError::StructureRedefined(name));
                    } else {
                        structs.insert(structure.get_name(), structure.clone());
                    }
                }
                MirDeclaration::Extern(filename) => externs.push(filename.clone()),
            }
        }

        for decl in decls {
            result.extend(decl.assemble(&mut funcs, &mut structs)?);
        }

        Ok(AsmProgram::new(externs, result, memory_size))
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirDeclaration {
    Structure(MirStructure),
    Function(MirFunction),
    Extern(PathBuf),
}

impl MirDeclaration {
    fn assemble(
        &self,
        funcs: &mut BTreeMap<Identifier, MirFunction>,
        structs: &mut BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmFunction>, MirError> {
        Ok(match self {
            Self::Structure(structure) => structure.assemble(funcs, structs)?,
            Self::Function(func) => vec![func.assemble(funcs, structs)?],
            _ => vec![],
        })
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct MirStructure {
    name: Identifier,
    size: i32,
    methods: Vec<MirFunction>,
}

impl MirStructure {
    pub fn new(name: Identifier, size: i32, methods: Vec<MirFunction>) -> Self {
        Self {
            name,
            size,
            methods,
        }
    }

    fn to_mir_type(&self) -> MirType {
        MirType::structure(self.name.clone())
    }

    fn get_name(&self) -> Identifier {
        self.name.clone()
    }

    fn get_size(&self) -> i32 {
        self.size
    }

    fn assemble(
        &self,
        funcs: &mut BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmFunction>, MirError> {
        // Check to see if this type redefines a primitive type
        match self.name.as_str() {
            MirType::BOOLEAN | MirType::CHAR | MirType::FLOAT | MirType::VOID => {
                return Err(MirError::PrimitiveTypeRedefined(self.name.clone()))
            }
            _ => {}
        }

        let mir_type = self.to_mir_type();
        let mut result = Vec::new();

        // Iterate over the methods and rename them
        // to their method names, such as `Date::day`
        for function in &self.methods {
            let method = function.as_method(&mir_type);
            funcs.insert(method.get_name(), method.clone());
        }

        // After each function has been declared, go back and assemble them.
        // We do two passes to allow methods to depend on one another.
        for function in &self.methods {
            result.push(function.as_method(&mir_type).assemble(funcs, structs)?);
        }

        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct MirFunction {
    name: Identifier,
    args: Vec<(Identifier, MirType)>,
    return_type: MirType,
    body: Vec<MirStatement>,
}

impl MirFunction {
    pub fn new(
        name: Identifier,
        args: Vec<(Identifier, MirType)>,
        return_type: MirType,
        body: Vec<MirStatement>,
    ) -> Self {
        Self {
            name,
            args,
            return_type,
            body,
        }
    }

    /// Convert this function to a method of a structure.
    /// This essentially renames the function to:
    /// `STRUCTURE_NAME::FUNCTION_NAME`
    fn as_method(&self, mir_type: &MirType) -> Self {
        let mut result = self.clone();
        result.name = mir_type.method_to_function_name(&self.name);
        result
    }

    fn assemble(
        &self,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<AsmFunction, MirError> {
        let mut asm_args = Vec::new();
        let mut vars = BTreeMap::new();
        for (arg_name, arg_type) in &self.args {
            // Add the arguments to the function's arguments, and to the map of variables.
            // The map of variables are not used to determine the function's stack size
            // at compile time, but are used to for resolving types.
            asm_args.push((arg_name.clone(), arg_type.to_asm_type(structs)?));
            vars.insert(arg_name.clone(), arg_type.clone());
        }

        // Assemble each statement in the body
        let mut asm_body = Vec::new();
        for stmt in &self.body {
            asm_body.extend(stmt.assemble(&mut vars, funcs, structs)?);
            stmt.type_check(&vars, funcs, structs)?
        }

        // Check return type
        let mut has_returned = false;
        for (i, stmt) in self.body.iter().enumerate() {
            if let MirStatement::Return(exprs) = stmt {
                // If the function has already used a return statement,
                // throw an error.
                if has_returned {
                    return Err(MirError::MultipleReturns(self.name.clone()));
                }
                has_returned = true;

                // Get the size of the return statement's stack allocation
                let mut result_size = 0;
                for expr in exprs {
                    result_size += expr.get_type(&vars, funcs, structs)?.get_size(structs)?;
                }

                // If the result's size is not equal to the size of the
                // return type, throw a type error.
                if result_size != self.return_type.get_size(structs)? {
                    return Err(MirError::MismatchedReturnType(self.name.clone()));

                // If there is only one return argument, check the individual
                // expression's type against the return type.
                } else if exprs.len() == 1
                    && self.return_type != exprs[0].get_type(&vars, funcs, structs)?
                {
                    return Err(MirError::MismatchedReturnType(self.name.clone()));
                }
            // If a statement has a return statement, but is not a return
            // statement itself, it must be a conditional return statement
            } else if stmt.has_return() {
                return Err(MirError::ConditionalReturn(self.name.clone()));
            }
        }

        // If the function is non-void and has not returned,
        // then throw an error.
        if !has_returned && self.return_type != MirType::void() {
            return Err(MirError::NonVoidNoReturn(self.name.clone()));
        }

        Ok(AsmFunction::new(
            self.name.clone(),
            asm_args,
            self.return_type.to_asm_type(structs)?,
            asm_body,
        ))
    }

    fn get_name(&self) -> Identifier {
        self.name.clone()
    }

    fn get_parameters(&self) -> Vec<(Identifier, MirType)> {
        self.args.clone()
    }

    fn get_return_type(&self) -> MirType {
        self.return_type.clone()
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirStatement {
    Define(Identifier, MirType, MirExpression),
    AutoDefine(Identifier, MirExpression),
    AssignVariable(Identifier, MirExpression),
    AssignAddress(MirExpression, MirExpression),

    For(Box<Self>, MirExpression, Box<Self>, Vec<Self>),
    While(MirExpression, Vec<Self>),
    If(MirExpression, Vec<Self>),
    IfElse(MirExpression, Vec<Self>, Vec<Self>),

    Free(MirExpression, MirExpression),
    Return(Vec<MirExpression>),
    Expression(MirExpression),
}

impl MirStatement {
    fn get_type(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<MirType, MirError> {
        if let Self::Expression(expr) = self {
            expr.get_type(vars, funcs, structs)
        } else {
            Ok(MirType::void())
        }
    }

    /// Does this statement eventually result in a return statement?
    fn has_return(&self) -> bool {
        match self {
            Self::For(pre, _, post, body) => {
                let mut result = false;
                for stmt in body {
                    result = result || stmt.has_return();
                }
                result || pre.has_return() || post.has_return()
            }

            Self::While(_, body) => {
                for stmt in body {
                    if stmt.has_return() {
                        return true;
                    }
                }
                false
            }

            Self::If(_, body) => {
                for stmt in body {
                    if stmt.has_return() {
                        return true;
                    }
                }
                false
            }

            Self::IfElse(_, then_body, else_body) => {
                for stmt in then_body {
                    if stmt.has_return() {
                        return true;
                    }
                }
                for stmt in else_body {
                    if stmt.has_return() {
                        return true;
                    }
                }
                false
            }

            Self::Return(_) => true,
            _ => false,
        }
    }

    /// Type check the MIR before it is lowered
    fn type_check(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<(), MirError> {
        match self {
            Self::Define(var_name, t, expr) => {
                expr.type_check(vars, funcs, structs)?;
                let rhs_type = expr.get_type(vars, funcs, structs)?;
                // Check to see if the defined type is equal to the type
                // of the right hand side of the assignment
                if t != &rhs_type {
                    // Return a mismatched type error
                    return Err(MirError::DefineMismatchedType(var_name.clone()));
                }
            }

            Self::AutoDefine(var_name, expr) => {
                expr.type_check(vars, funcs, structs)?;
                let t = expr.get_type(vars, funcs, structs)?;
                // Let expressions MUST cast void pointers.
                // This error catches code like `let ptr = alloc(10)`
                if t.is_void_ptr() {
                    return Err(MirError::AutoDefineVoidPointer(
                        var_name.clone(),
                        expr.clone(),
                    ));
                }
            }

            Self::AssignAddress(lhs, rhs) => {
                lhs.type_check(vars, funcs, structs)?;
                rhs.type_check(vars, funcs, structs)?;
                let lhs_type = lhs.get_type(vars, funcs, structs)?;
                let rhs_type = rhs.get_type(vars, funcs, structs)?;

                // Compare the left hand side and right hand side
                // If the LHS is a void pointer, allow the assignment.
                // If the type *LHS is equal to RHS, also allow the assignment.
                if lhs_type != MirType::void().refer() && lhs_type.deref()? != rhs_type {
                    // Return a mismatched type error
                    return Err(MirError::AssignMismatchedType(lhs.clone()));
                }
            }

            Self::AssignVariable(var_name, rhs) => {
                rhs.type_check(vars, funcs, structs)?;
                let rhs_type = rhs.get_type(vars, funcs, structs)?;

                // Check to see if the variable has been defined
                if let Some(lhs_type) = vars.get(var_name) {
                    // Check the LHS and RHS types
                    if lhs_type != &rhs_type {
                        // Return a mismatched type error
                        return Err(MirError::AssignMismatchedType(MirExpression::Variable(
                            var_name.clone(),
                        )));
                    }
                } else {
                    return Err(MirError::VariableNotDefined(var_name.clone()));
                }
            }

            Self::For(pre, cond, post, body) => {
                pre.type_check(vars, funcs, structs)?;
                cond.type_check(vars, funcs, structs)?;
                post.type_check(vars, funcs, structs)?;

                // Confirm the condition is a boolean
                if cond.get_type(vars, funcs, structs)? != MirType::boolean() {
                    return Err(MirError::NonBooleanCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::While(cond, body) => {
                cond.type_check(vars, funcs, structs)?;

                // Confirm the condition is a boolean
                if cond.get_type(vars, funcs, structs)? != MirType::boolean() {
                    return Err(MirError::NonBooleanCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::If(cond, body) => {
                cond.type_check(vars, funcs, structs)?;

                // Confirm the condition is a boolean
                if cond.get_type(vars, funcs, structs)? != MirType::boolean() {
                    return Err(MirError::NonBooleanCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::IfElse(cond, then_body, else_body) => {
                cond.type_check(vars, funcs, structs)?;

                // Confirm the condition is a boolean
                if cond.get_type(vars, funcs, structs)? != MirType::boolean() {
                    return Err(MirError::NonBooleanCondition(cond.clone()));
                }

                for stmt in then_body {
                    stmt.type_check(vars, funcs, structs)?
                }
                for stmt in else_body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::Return(exprs) => {
                for expr in exprs {
                    expr.type_check(vars, funcs, structs)?
                }
            }

            Self::Free(address, size) => {
                address.type_check(vars, funcs, structs)?;
                size.type_check(vars, funcs, structs)?;

                // If the address is a non-pointer, return an error
                if !address.get_type(vars, funcs, structs)?.is_pointer() {
                    return Err(MirError::FreeNonPointer(address.clone()));
                }
            }

            Self::Expression(expr) => {
                expr.type_check(vars, funcs, structs)?;
                if let MirExpression::ForeignCall(_, _) = expr {
                    // If the expression is a foreign call, then we
                    // trust that the user is calling a void foreign
                    // function.
                } else if expr.get_type(vars, funcs, structs)?.get_size(structs)? != 0 {
                    return Err(MirError::NonVoidExpressionNotUsed(expr.clone()));
                }
            }
        }
        Ok(())
    }

    /// Lower MIR into Oak's ASM
    fn assemble(
        &self,
        vars: &mut BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmStatement>, MirError> {
        Ok(match self {
            /// Define a variable with a given type
            Self::Define(var_name, t, expr) => {
                // Add the variable to the defined variables in the scope
                vars.insert(var_name.clone(), t.clone());
                let mut result = Vec::new();
                let asm_t = t.to_asm_type(structs)?;

                // Push the expression to store in the variable
                result.extend(expr.assemble(vars, funcs, structs)?);
                // Allocate the variable on the stack, and store the
                // expression at the variable's new address
                result.extend(vec![
                    AsmStatement::Define(var_name.clone(), asm_t),
                    AsmStatement::Assign(asm_t),
                ]);
                result
            }

            /// A let statement that automatically deduces the type
            /// of the variable just expands to a manually defined MIR let statement.
            Self::AutoDefine(var_name, expr) => Self::Define(
                var_name.clone(),
                expr.get_type(vars, funcs, structs)?,
                expr.clone(),
            )
            .assemble(vars, funcs, structs)?,

            /// Assign an expression to a defined variable
            Self::AssignVariable(var_name, expr) => {
                // Check to see if the variable has been defined
                if let Some(t) = vars.clone().get(var_name) {
                    let mut result = Vec::new();
                    // Push the expression to store onto the stack
                    result.extend(expr.assemble(vars, funcs, structs)?);
                    // Store the expression at the address of the variable
                    result.extend(vec![
                        AsmStatement::Expression(vec![AsmExpression::Refer(var_name.clone())]),
                        AsmStatement::Assign(t.to_asm_type(structs)?),
                    ]);
                    result
                } else {
                    return Err(MirError::VariableNotDefined(var_name.clone()));
                }
            }

            /// Dereference an address and store an expression there.
            /// This is equivalent to the C code: `*ptr = expr`
            Self::AssignAddress(lhs, rhs) => {
                let mut result = Vec::new();
                // Push the expression to store onto the stack
                result.extend(rhs.assemble(vars, funcs, structs)?);
                // Push the address to dereference onto the stack
                result.extend(lhs.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Assign(
                    rhs.get_type(vars, funcs, structs)?.to_asm_type(structs)?,
                ));
                result
            }

            Self::For(pre, cond, post, body) => {
                // Assemble the `pre` condition first so that
                // if a variable is defined in this statement,
                // it is defined for the rest of the loop.
                let asm_pre = pre.assemble(vars, funcs, structs)?;
                let mut asm_body = Vec::new();
                for stmt in body {
                    asm_body.extend(stmt.assemble(vars, funcs, structs)?);
                }
                vec![AsmStatement::For(
                    asm_pre,
                    cond.assemble(vars, funcs, structs)?,
                    post.assemble(vars, funcs, structs)?,
                    asm_body,
                )]
            }

            Self::While(cond, body) => {
                let mut asm_body = Vec::new();
                for stmt in body {
                    asm_body.extend(stmt.assemble(vars, funcs, structs)?);
                }
                // Create a for loop using only a condition.
                vec![AsmStatement::For(
                    vec![],
                    cond.assemble(vars, funcs, structs)?,
                    vec![],
                    asm_body,
                )]
            }

            Self::If(cond, body) => {
                let mut asm_body = Vec::new();
                for stmt in body {
                    asm_body.extend(stmt.assemble(vars, funcs, structs)?);
                }

                // Use a variable to store the condition of the if statement
                let mut pre = Vec::new();
                pre.extend(cond.assemble(vars, funcs, structs)?);
                pre.extend(vec![
                    AsmStatement::Define(Identifier::from("%IF_VAR%"), AsmType::float()),
                    AsmStatement::Assign(AsmType::float()),
                ]);

                // At the end of the loop body, store zero in the condition variable
                // to prevent the statement from doing more than one loop.
                let mut post = Vec::new();
                post.extend(vec![
                    AsmStatement::Expression(vec![
                        AsmExpression::Float(0.0),
                        AsmExpression::Refer(Identifier::from("%IF_VAR%")),
                    ]),
                    AsmStatement::Assign(AsmType::float()),
                ]);
                vec![AsmStatement::For(
                    pre,
                    vec![AsmStatement::Expression(vec![AsmExpression::Variable(
                        Identifier::from("%IF_VAR%"),
                    )])],
                    post,
                    asm_body,
                )]
            }

            Self::IfElse(cond, then_body, else_body) => {
                let mut asm_then_body = Vec::new();
                for stmt in then_body {
                    asm_then_body.extend(stmt.assemble(vars, funcs, structs)?);
                }

                let mut asm_else_body = Vec::new();
                for stmt in else_body {
                    asm_else_body.extend(stmt.assemble(vars, funcs, structs)?);
                }

                // Use a variable to store the condition of the if statement
                let mut pre = Vec::new();
                pre.extend(cond.assemble(vars, funcs, structs)?);
                pre.extend(vec![
                    AsmStatement::Define(Identifier::from("%IF_VAR%"), AsmType::float()),
                    AsmStatement::Assign(AsmType::float()),
                    AsmStatement::Expression(vec![AsmExpression::Float(1.0)]),
                    AsmStatement::Define(Identifier::from("%ELSE_VAR%"), AsmType::float()),
                    AsmStatement::Assign(AsmType::float()),
                ]);

                // At the end of the loop body, store zero in the condition variable
                // to prevent the statement from doing more than one loop.
                let mut post = Vec::new();
                post.extend(vec![
                    AsmStatement::Expression(vec![
                        AsmExpression::Float(0.0),
                        AsmExpression::Refer(Identifier::from("%IF_VAR%")),
                    ]),
                    AsmStatement::Assign(AsmType::float()),
                    AsmStatement::Expression(vec![
                        AsmExpression::Float(0.0),
                        AsmExpression::Refer(Identifier::from("%ELSE_VAR%")),
                    ]),
                    AsmStatement::Assign(AsmType::float()),
                ]);

                // The resulting code for an if-else statement!
                vec![
                    AsmStatement::For(
                        pre,
                        vec![AsmStatement::Expression(vec![AsmExpression::Variable(
                            Identifier::from("%IF_VAR%"),
                        )])],
                        post.clone(),
                        asm_then_body,
                    ),
                    AsmStatement::For(
                        vec![],
                        vec![AsmStatement::Expression(vec![AsmExpression::Variable(
                            Identifier::from("%ELSE_VAR%"),
                        )])],
                        post,
                        asm_else_body,
                    ),
                ]
            }

            Self::Return(exprs) => {
                let mut result = Vec::new();
                for expr in exprs {
                    result.extend(expr.assemble(vars, funcs, structs)?)
                }
                result
            }

            /// Freeing an address does not return a value, so it is a statement.
            Self::Free(addr, size) => {
                let mut result = Vec::new();
                result.extend(size.assemble(vars, funcs, structs)?);
                result.extend(addr.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Free]));
                result
            }

            Self::Expression(expr) => expr.assemble(vars, funcs, structs)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirExpression {
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

    String(StringLiteral),
    Float(f64),
    Character(char),
    True,
    False,
    Void,

    Variable(Identifier),
    Refer(Identifier),
    Deref(Box<Self>),

    TypeCast(Box<Self>, MirType),
    Alloc(Box<Self>),

    Call(Identifier, Vec<Self>),
    ForeignCall(Identifier, Vec<Self>),
    Method(Box<Self>, Identifier, Vec<Self>),
    Index(Box<Self>, Box<Self>),
}

impl MirExpression {
    fn type_check(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<(), MirError> {
        match self {
            // Typecheck a typecast
            Self::TypeCast(expr, t) => {
                expr.type_check(vars, funcs, structs)?;

                // If the expression and cast type have different sizes,
                // then the expression cannot be cast to this type.
                if expr.get_type(vars, funcs, structs)?.get_size(structs) != t.get_size(structs) {
                    return Err(MirError::MismatchedCastSize(*expr.clone(), t.clone()));
                }
            }

            Self::Not(expr) => {
                expr.type_check(vars, funcs, structs)?;
                let expr_type = expr.get_type(vars, funcs, structs)?;
                if expr_type.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberUnaryOperation(*expr.clone()));
                }
            }

            // Typecheck binary operations
            // Currently, type checking only fails if either the left hand side
            // or the right hand side are of type `void`, or a user defined structure
            Self::Add(lhs, rhs)
            | Self::Subtract(lhs, rhs)
            | Self::Multiply(lhs, rhs)
            | Self::Divide(lhs, rhs)
            | Self::Greater(lhs, rhs)
            | Self::Less(lhs, rhs)
            | Self::GreaterEqual(lhs, rhs)
            | Self::LessEqual(lhs, rhs)
            | Self::Equal(lhs, rhs)
            | Self::NotEqual(lhs, rhs)
            | Self::And(lhs, rhs)
            | Self::Or(lhs, rhs) => {
                lhs.type_check(vars, funcs, structs)?;
                rhs.type_check(vars, funcs, structs)?;
                let lhs_type = lhs.get_type(vars, funcs, structs)?;
                let rhs_type = rhs.get_type(vars, funcs, structs)?;
                if lhs_type.get_size(structs)? != 1 || rhs_type.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberBinaryOperation(
                        *lhs.clone(),
                        *rhs.clone(),
                    ));
                }
            }

            // Typecheck an `alloc` expression
            Self::Alloc(size_expr) => {
                size_expr.type_check(vars, funcs, structs)?;
                if size_expr.get_type(vars, funcs, structs)? != MirType::float() {
                    return Err(MirError::NonNumberAllocate(*size_expr.clone()));
                }
            }

            // Typecheck an index expression
            Self::Index(ptr, idx) => {
                ptr.type_check(vars, funcs, structs)?;
                idx.type_check(vars, funcs, structs)?;

                // Check if the index is a structure or of type `void`
                if idx.get_type(vars, funcs, structs)?.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberIndex(*idx.clone()));
                }

                // Check to see if the pointer being indexed is a void pointer
                if ptr
                    .get_type(vars, funcs, structs)?
                    .deref()?
                    .get_size(structs)?
                    == 0
                {
                    return Err(MirError::IndexVoidPointer(*ptr.clone()));
                }
            }

            // Typecheck a function call expression
            Self::Call(fn_name, args) => {
                // Get the function structure
                if let Some(func) = funcs.get(fn_name) {
                    // The list of parameters that the function expects
                    let params = func.get_parameters();

                    // Check if there are too many or few arguments
                    if args.len() < params.len() {
                        return Err(MirError::NotEnoughArguments(self.clone()));
                    } else if args.len() > params.len() {
                        return Err(MirError::TooManyArguments(self.clone()));
                    }

                    // Iterate over the function's parameters and the list of arguments
                    // to the function call
                    for ((_, param_type), arg_expr) in func.get_parameters().iter().zip(args) {
                        // If the parameters don't match the argument types,
                        // then throw an error.
                        if param_type != &arg_expr.get_type(vars, funcs, structs)? {
                            return Err(MirError::ArgumentMismatchedType(self.clone()));
                        }
                    }
                } else {
                    return Err(MirError::FunctionNotDefined(fn_name.clone()));
                }
            }

            // Typecheck a method call expression
            Self::Method(expr, method_name, args) => {
                // Get the type of the object
                let instance_type = expr.get_type(vars, funcs, structs)?;
                // Get the name of the method
                let fn_name = instance_type.method_to_function_name(method_name);

                if let Some(func) = funcs.get(&fn_name) {
                    // The list of parameters that the function expects
                    let mut params = func.get_parameters();

                    if let Some((_, self_type)) = params.first() {
                        // If the first parameter of the method ISN'T a pointer,
                        // then the function is not a method. It's an associated function,
                        // like: `fn new(m: num, d: num, y: num) -> Date { m; d; y }`
                        if !self_type.is_pointer() {
                            return Err(MirError::CalledFunctionAsMethod(fn_name.clone()));
                        }

                        // Get rid of the `self` parameter
                        let _ = params.remove(0);

                        // Check if there are too many or few arguments
                        if args.len() < params.len() {
                            return Err(MirError::NotEnoughArguments(self.clone()));
                        } else if args.len() > params.len() {
                            return Err(MirError::TooManyArguments(self.clone()));
                        }

                        // Iterate over the methods's parameters and the list of arguments
                        for ((_, param_type), arg) in params.iter().zip(args) {
                            // If the parameters don't match the argument types,
                            // then throw an error.
                            if param_type != &arg.get_type(vars, funcs, structs)? {
                                return Err(MirError::ArgumentMismatchedType(self.clone()));
                            }
                        }
                    } else {
                        return Err(MirError::CalledFunctionAsMethod(fn_name.clone()));
                    }
                } else {
                    return Err(MirError::FunctionNotDefined(fn_name.clone()));
                }
            }

            // Typecheck a dereference expression
            Self::Deref(expr) => expr.type_check(vars, funcs, structs)?,

            // Typecheck atomic expressions
            Self::ForeignCall(_, _)
            | Self::Refer(_)
            | Self::Variable(_)
            | Self::String(_)
            | Self::Float(_)
            | Self::Character(_)
            | Self::Void
            | Self::True
            | Self::False => {}
        }
        Ok(())
    }

    fn assemble(
        &self,
        vars: &mut BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmStatement>, MirError> {
        Ok(match self {
            Self::True => vec![AsmStatement::Expression(vec![AsmExpression::Float(1.0)])],
            Self::False => vec![AsmStatement::Expression(vec![AsmExpression::Float(0.0)])],

            // Invert the boolean value of an expression
            Self::Not(expr) => MirStatement::IfElse(
                *expr.clone(),
                vec![MirStatement::Expression(MirExpression::Float(0.0))],
                vec![MirStatement::Expression(MirExpression::Float(1.0))],
            )
            .assemble(vars, funcs, structs)?,

            /// And two boolean values
            /// And is essentially boolean multiplication,
            /// so multiply these two values and use it
            /// as a condition for which value to use
            Self::And(l, r) => MirStatement::IfElse(
                MirExpression::Multiply(l.clone(), r.clone()),
                vec![MirStatement::Expression(MirExpression::Float(1.0))],
                vec![MirStatement::Expression(MirExpression::Float(0.0))],
            )
            .assemble(vars, funcs, structs)?,

            /// Or two boolean values
            /// Or is essentially boolean addition,
            /// so add these two values and use it
            /// as a condition for which value to use
            Self::Or(l, r) => MirStatement::IfElse(
                MirExpression::Add(l.clone(), r.clone()),
                vec![MirStatement::Expression(MirExpression::Float(1.0))],
                vec![MirStatement::Expression(MirExpression::Float(0.0))],
            )
            .assemble(vars, funcs, structs)?,

            /// Are two numbers equal?
            /// I know this expression doesn't type check,
            /// but it is correctly implemented.
            Self::Equal(l, r) => MirStatement::IfElse(
                MirExpression::Subtract(l.clone(), r.clone()),
                vec![MirStatement::Expression(MirExpression::Float(0.0))],
                vec![MirStatement::Expression(MirExpression::Float(1.0))],
            )
            .assemble(vars, funcs, structs)?,

            /// Are two numbers not equal?
            /// I know this expression doesn't type check,
            /// but it is correctly implemented.
            Self::NotEqual(l, r) => MirStatement::IfElse(
                MirExpression::Subtract(l.clone(), r.clone()),
                vec![MirStatement::Expression(MirExpression::Float(1.0))],
                vec![MirStatement::Expression(MirExpression::Float(0.0))],
            )
            .assemble(vars, funcs, structs)?,

            /// A typecast is only a way to explicitly validate
            /// some kinds of typechecks. The typecast expression
            /// has no change on the output code.
            Self::TypeCast(expr, _) => expr.assemble(vars, funcs, structs)?,

            /// Is the LHS greater than or equal the RHS?
            Self::GreaterEqual(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![
                    // Subtract RHS from the LHS and check the sign
                    AsmExpression::Subtract,
                    AsmExpression::Sign,
                    // If the sign was 1, then this expression is true.
                    AsmExpression::Float(1.0),
                    AsmExpression::Add,
                    AsmExpression::Float(2.0),
                    AsmExpression::Divide,
                ]));
                result
            }
            /// Is the LHS greater than the RHS?
            Self::Greater(l, r) => {
                let mut result = Vec::new();
                result.extend(r.assemble(vars, funcs, structs)?);
                result.extend(l.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![
                    // Subtract LHS from the RHS and check the sign
                    AsmExpression::Subtract,
                    AsmExpression::Sign,
                    // If the sign was -1, then this expression is true.
                    AsmExpression::Float(1.0),
                    AsmExpression::Subtract,
                    AsmExpression::Float(-2.0),
                    AsmExpression::Divide,
                ]));
                result
            }
            /// Is the LHS less than or equal to the RHS?
            Self::LessEqual(l, r) => {
                let mut result = Vec::new();
                result.extend(r.assemble(vars, funcs, structs)?);
                result.extend(l.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![
                    // Subtract LHS from the RHS and check the sign
                    AsmExpression::Subtract,
                    AsmExpression::Sign,
                    // If the sign was 1, then this expression is true.
                    AsmExpression::Float(1.0),
                    AsmExpression::Add,
                    AsmExpression::Float(2.0),
                    AsmExpression::Divide,
                ]));
                result
            }
            /// Is the LHS less than the RHS?
            Self::Less(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![
                    // Subtract RHS from the LHS and check the sign
                    AsmExpression::Subtract,
                    AsmExpression::Sign,
                    // If the sign was -1, then this expression is true.
                    AsmExpression::Float(1.0),
                    AsmExpression::Subtract,
                    AsmExpression::Float(-2.0),
                    AsmExpression::Divide,
                ]));
                result
            }

            /// Add two values
            Self::Add(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Add]));
                result
            }
            /// Multiply two values
            Self::Multiply(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Multiply]));
                result
            }
            /// Divide two values
            Self::Divide(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Divide]));
                result
            }
            /// Subtract two values
            Self::Subtract(l, r) => {
                let mut result = Vec::new();
                result.extend(l.assemble(vars, funcs, structs)?);
                result.extend(r.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Subtract]));
                result
            }

            /// Push the address of a string literal onto the stack
            Self::String(s) => vec![AsmStatement::Expression(vec![AsmExpression::String(
                s.clone(),
            )])],
            /// Push a float onto the stack
            Self::Float(n) => vec![AsmStatement::Expression(vec![AsmExpression::Float(*n)])],
            /// Push a character on the stack
            Self::Character(ch) => vec![AsmStatement::Expression(vec![AsmExpression::Character(
                *ch,
            )])],
            /// Void expression (No-op)
            Self::Void => vec![AsmStatement::Expression(vec![AsmExpression::Void])],
            /// Load data from a variable on the stack
            Self::Variable(var_name) => {
                vec![AsmStatement::Expression(vec![AsmExpression::Variable(
                    var_name.clone(),
                )])]
            }
            /// Reference a variable on the stack
            Self::Refer(var_name) => vec![AsmStatement::Expression(vec![AsmExpression::Refer(
                var_name.clone(),
            )])],
            /// Dereference a pointer
            Self::Deref(expr) => {
                let mut result = Vec::new();
                result.extend(expr.assemble(vars, funcs, structs)?);
                // The `Deref` instruction requires the size of the item in memory
                // to push onto the stack. A pointer to the object has size 1, but
                // the size of the type itself can vary. To get the size of the
                // inner type, dereference the pointer type and get the size of
                // the resulting type.
                result.push(AsmStatement::Expression(vec![AsmExpression::Deref(
                    expr.get_type(vars, funcs, structs)?
                        .deref()?
                        .get_size(structs)?,
                )]));
                result
            }

            /// Call a user defined function
            Self::Call(func_name, args) => {
                let mut result = Vec::new();
                // Push arguments onto the stack in reverse order
                for arg in args.iter().rev() {
                    result.extend(arg.assemble(vars, funcs, structs)?);
                }
                // Call the function
                result.push(AsmStatement::Expression(vec![AsmExpression::Call(
                    func_name.clone(),
                )]));
                result
            }

            /// Call a foreign function
            Self::ForeignCall(func_name, args) => {
                let mut result = Vec::new();
                for arg in args.iter().rev() {
                    result.extend(arg.assemble(vars, funcs, structs)?);
                }
                result.push(AsmStatement::Expression(vec![AsmExpression::ForeignCall(
                    func_name.clone(),
                )]));
                result
            }

            /// Allocate data on the heap
            Self::Alloc(size_expr) => {
                let mut result = Vec::new();
                result.extend(size_expr.assemble(vars, funcs, structs)?);
                result.push(AsmStatement::Expression(vec![AsmExpression::Alloc]));
                result
            }

            /// Call a method on an object
            Self::Method(expr, method_name, args) => {
                let instance_type = expr.get_type(vars, funcs, structs)?;
                let func_name = instance_type.method_to_function_name(method_name);

                // If the instance object is already a pointer, call the
                // method with the pointer to the object without referencing it again.
                if expr.get_type(vars, funcs, structs)?.is_pointer() {
                    let mut call_args = vec![*expr.clone()];
                    call_args.extend(args.clone());
                    return Self::Call(func_name, call_args).assemble(vars, funcs, structs);
                // Here the instance object must be a non-pointer type
                // and also a variable. In this case, reference the
                // variable and call the method with the pointer to the object.
                } else if let Self::Variable(name) = *expr.clone() {
                    // Reference the variable storing the object
                    let mut call_args = vec![Self::Refer(name.clone())];
                    call_args.extend(args.clone());
                    Self::Call(func_name, call_args).assemble(vars, funcs, structs)?
                // Here, the instance object must be an object stored on the stack
                // at an address not known at compile time. This case is much more complicated.
                // In this case,
                } else {
                    let mut result = Vec::new();
                    // Push the instance object
                    result.extend(expr.assemble(vars, funcs, structs)?);

                    let self_type = instance_type.to_asm_type(structs)?;
                    result.extend(vec![
                        // Store the instance object into a stack variable
                        AsmStatement::Define(Identifier::from("%INSTANCE_VAR%"), self_type),
                        AsmStatement::Assign(self_type),
                    ]);

                    let mut call_args = vec![Self::Refer(Identifier::from("%INSTANCE_VAR%"))];
                    call_args.extend(args.clone());

                    result.extend(Self::Call(func_name, call_args).assemble(vars, funcs, structs)?);

                    result
                }
            }

            /// Assemble the MIR code for indexing a pointer.
            Self::Index(ptr, idx) => {
                let mut result = Vec::new();
                // Push the array pointer on the stack
                result.extend(ptr.assemble(vars, funcs, structs)?);
                // Push the index of the array onto the stack
                result.extend(idx.assemble(vars, funcs, structs)?);
                // Get the size of the array's inner type
                let type_size = ptr
                    .get_type(vars, funcs, structs)?
                    .deref()?
                    .get_size(structs)?;
                // Multiply the index and the size of the inner type,
                // then add this offset to the array pointer. This is
                // the address of the indexed item in the array.
                result.push(AsmStatement::Expression(vec![
                    AsmExpression::Float(type_size as f64),
                    AsmExpression::Multiply,
                    AsmExpression::Add,
                ]));
                result
            }
        })
    }

    fn get_type(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<MirType, MirError> {
        Ok(match self {
            Self::True => MirType::boolean(),
            Self::False => MirType::boolean(),

            /// A typecast simply masks the type of the cast expression.
            /// The typecast has the type of whichever type the
            /// expression is being cast to.
            Self::TypeCast(_, t) => t.clone(),

            /// Arithmetic returns the type of the left hand side
            Self::Add(l, _) | Self::Subtract(l, _) | Self::Multiply(l, _) | Self::Divide(l, _) => {
                l.get_type(vars, funcs, structs)?
            }
            /// Greater than, less than, greater or equal,
            /// and less than or equal expressions ALL return
            /// boolean values.
            Self::Greater(_, _)
            | Self::Less(_, _)
            | Self::GreaterEqual(_, _)
            | Self::LessEqual(_, _)
            | Self::Equal(_, _)
            | Self::NotEqual(_, _)
            | Self::And(_, _)
            | Self::Or(_, _)
            | Self::Not(_) => MirType::boolean(),
            /// Float literals have type `num`
            Self::Float(_) => MirType::float(),
            /// String literals have type `&char`
            Self::String(_) => MirType::character().refer(),
            /// char literals have type `char`
            Self::Character(_) => MirType::character(),
            /// A void literal has type `void`
            Self::Void => MirType::void(),
            /// Allocating data on the heap returns a void pointer
            Self::Alloc(_) => MirType::void().refer(),

            /// Get the type of the instance, retrieve the method from the type,
            /// then get the return type of the method.
            Self::Method(expr, method_name, _) => {
                // Get the type of the object
                let mut instance_type = expr.get_type(vars, funcs, structs)?;
                while instance_type.is_pointer() {
                    instance_type = instance_type.deref()?
                }
                // Get the return type of the method
                let func_name = instance_type.method_to_function_name(method_name);
                if let Some(func) = funcs.get(&func_name) {
                    func.get_return_type()
                } else {
                    return Err(MirError::MethodNotDefined(
                        instance_type,
                        method_name.clone(),
                    ));
                }
            }

            /// When a pointer is indexed, the resulting type is
            /// a pointer of the same type. This is because indexing
            /// a pointer returns the address of the object in the array.
            Self::Index(ptr, _) => ptr.get_type(vars, funcs, structs)?,

            /// Get the return type of the called function
            Self::Call(func_name, _) => {
                if let Some(func) = funcs.get(func_name) {
                    func.get_return_type()
                } else {
                    return Err(MirError::FunctionNotDefined(func_name.clone()));
                }
            }

            /// The type of foreign functions are unknown. The type system
            /// assumes they are of type &void.
            Self::ForeignCall(_, _) => MirType::void().refer(),

            /// Get the type of the variable
            Self::Variable(var_name) => {
                if let Some(t) = vars.get(var_name) {
                    t.clone()
                } else {
                    return Err(MirError::VariableNotDefined(var_name.clone()));
                }
            }

            /// Get the type of the dereferenced expression and remove a reference
            Self::Deref(outer) => outer.get_type(vars, funcs, structs)?.deref()?,

            /// Get the type of the variable, and add a reference to it.
            Self::Refer(var_name) => {
                if let Some(t) = vars.get(var_name) {
                    t.refer()
                } else {
                    return Err(MirError::VariableNotDefined(var_name.clone()));
                }
            }
        })
    }
}

impl Display for MirExpression {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::TypeCast(expr, t) => write!(f, "{} as {}", expr, t),

            Self::Not(expr) => write!(f, "!{}", expr),
            Self::And(lhs, rhs) => write!(f, "{}&&{}", lhs, rhs),
            Self::Or(lhs, rhs) => write!(f, "{}||{}", lhs, rhs),

            Self::Add(lhs, rhs) => write!(f, "{}+{}", lhs, rhs),
            Self::Subtract(lhs, rhs) => write!(f, "{}-{}", lhs, rhs),
            Self::Multiply(lhs, rhs) => write!(f, "{}/{}", lhs, rhs),
            Self::Divide(lhs, rhs) => write!(f, "{}/{}", lhs, rhs),

            Self::Equal(lhs, rhs) => write!(f, "{}=={}", lhs, rhs),
            Self::NotEqual(lhs, rhs) => write!(f, "{}!={}", lhs, rhs),
            Self::Greater(lhs, rhs) => write!(f, "{}>{}", lhs, rhs),
            Self::GreaterEqual(lhs, rhs) => write!(f, "{}>={}", lhs, rhs),
            Self::Less(lhs, rhs) => write!(f, "{}<{}", lhs, rhs),
            Self::LessEqual(lhs, rhs) => write!(f, "{}<={}", lhs, rhs),

            Self::Alloc(size) => write!(f, "alloc({})", size),

            Self::Void => write!(f, "@"),
            Self::Character(ch) => write!(f, "'{}'", ch),
            Self::Float(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{:?}", s),

            Self::Index(ptr, idx) => write!(f, "{}[{}]", ptr, idx),
            Self::Method(expr, method, args) => {
                write!(f, "{}.{}(", expr, method)?;
                for arg in args {
                    write!(f, "{}, ", arg)?;
                }
                write!(f, ")")
            }
            Self::Call(fn_name, args) => {
                write!(f, "{}(", fn_name)?;
                for arg in args {
                    write!(f, "{}, ", arg)?;
                }
                write!(f, ")")
            }
            Self::ForeignCall(fn_name, args) => {
                write!(f, "{}!(", fn_name)?;
                for arg in args {
                    write!(f, "{}, ", arg)?;
                }
                write!(f, ")")
            }
            Self::Deref(ptr) => write!(f, "*{}", ptr),
            Self::Refer(name) => write!(f, "&{}", name),
            Self::Variable(name) => write!(f, "{}", name),
        }
    }
}
