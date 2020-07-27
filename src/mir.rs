use std::{
    collections::BTreeMap,
    fmt::{Display, Error, Formatter},
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
    /// Defining a function multiple times
    FunctionRedefined(Identifier),
    /// Using a variable without declaring it
    VariableNotDefined(Identifier),
    /// Calling a method for a type where it is not defined
    MethodNotDefined(MirType, Identifier),
    /// Using a structure name as a type without defining it
    StructureNotDefined(Identifier),
    /// Dereferencing a value without a reference type
    DereferenceNonPointer(MirType),
    /// Mismatched types in a `let` statement
    DefineMismatchedType(String),
    /// Mismatched types in an assignment statement
    AssignMismatchedType(MirExpression),
    /// Arguments to a function call do not match parameter types
    ArgumentMismatchedType(MirExpression),
    /// Use a `free` statement using an address argument
    /// of a non-pointer type
    FreeNonPointer(MirExpression),
    /// Using a non-number for an if statement, and if-else
    /// statement, a while loop, or a for loop
    NonNumberCondition(MirExpression),
    /// Using a non-number for an `alloc` call
    NonNumberAllocate(MirExpression),
    /// Indexing an array with a non-number value
    NonNumberIndex(MirExpression),
    /// Adding, subtracting, multiplying, or dividing two
    /// values where one or more of them is not a number.
    NonNumberBinaryOperation(MirExpression, MirExpression),
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
}

/// Print an MIR error on the command line
impl Display for MirError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::FunctionNotDefined(name) => write!(f, "function '{}' is not defined", name),
            Self::FunctionRedefined(name) => {
                write!(f, "function '{}' is defined multiple times", name)
            }
            Self::VariableNotDefined(name) => write!(f, "variable '{}' is not defined", name),
            Self::MethodNotDefined(t, name) => {
                write!(f, "method '{}' is not defined for type '{}'", name, t)
            }
            Self::StructureNotDefined(name) => write!(f, "type '{}' is not defined", name),
            Self::DereferenceNonPointer(t) => write!(f, "cannot dereference type '{}'", t),

            Self::DefineMismatchedType(var_name) => {
                write!(f, "mismatched types when defining variable '{}'", var_name)
            }

            Self::AssignMismatchedType(lhs_expr) => {
                write!(f, "mismatched types when assigning to '{}'", lhs_expr)
            }
            Self::FreeNonPointer(address_expr) => {
                write!(f, "cannot free non-pointer '{}'", address_expr)
            }
            Self::NonNumberCondition(cond_expr) => {
                write!(f, "cannot use '{}' as a boolean condition", cond_expr)
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

    /// A user defined type
    pub fn structure(name: Identifier) -> Self {
        Self { name, ptr_level: 0 }
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
            "void" => 0,
            "num" => 1,
            "char" => 1,
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

    fn method_to_function_name(&self, method_name: &Identifier) -> Identifier {
        format!("{}::{}", self.name, method_name)
    }
}

/// This implementation solely governs the rules for type-checking.
impl PartialEq for MirType {
    fn eq(&self, other: &Self) -> bool {
        // If two types are identical, they are equal
        if self.name == other.name && self.ptr_level == other.ptr_level {
            true
        } else if !self.is_pointer() {
            // (char == num) AND (num == char)
            match (self.name.as_str(), other.name.as_str()) {
                ("char", "num") => true,
                ("num", "char") => true,
                _ => false,
            }
        } else {
            // (&void == &*) AND (&* == &void)
            (self.ptr_level == 1 && self.name == "void" && other.ptr_level == 1)
                || (other.ptr_level == 1 && other.name == "void" && other.ptr_level == 1)
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
    pub fn new(decls: Vec<MirDeclaration>, heap_size: i32) -> Self {
        Self(decls, heap_size)
    }

    pub fn get_declarations(&self) -> Vec<MirDeclaration> {
        (self.0).clone()
    }
    pub fn get_heap_size(&self) -> i32 {
        self.1
    }

    pub fn assemble(&self) -> Result<AsmProgram, MirError> {
        let Self(decls, heap_size) = self.clone();
        let mut funcs = BTreeMap::new();
        let mut structs = BTreeMap::new();
        let mut result = Vec::new();
        for decl in &decls {
            match decl {
                MirDeclaration::Function(func) => {
                    funcs.insert(func.get_name(), func.clone());
                }
                MirDeclaration::Structure(structure) => {
                    structs.insert(structure.get_name(), structure.clone());
                }
            }
        }

        for decl in decls {
            result.extend(decl.assemble(&mut funcs, &mut structs)?);
        }

        Ok(AsmProgram::new(result, heap_size))
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirDeclaration {
    Structure(MirStructure),
    Function(MirFunction),
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
        // if let Some(last_stmt) = self.body.last() {
        //     if self.return_type != last_stmt.get_type(&vars, funcs, structs)? {
        //         return Err(MirError::MismatchedReturnType(self.name.clone()))
        //     }
        // }

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

    /// Get the parameters
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
    AssignVariable(Identifier, MirExpression),
    AssignAddress(MirExpression, MirExpression),

    For(Box<Self>, MirExpression, Box<Self>, Vec<Self>),
    While(MirExpression, Vec<Self>),
    If(MirExpression, Vec<Self>),
    IfElse(MirExpression, Vec<Self>, Vec<Self>),

    Free(MirExpression, MirExpression),
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

            Self::AssignAddress(lhs, rhs) => {
                lhs.type_check(vars, funcs, structs)?;
                rhs.type_check(vars, funcs, structs)?;
                let lhs_type = lhs.get_type(vars, funcs, structs)?;
                let rhs_type = rhs.get_type(vars, funcs, structs)?;

                // Compare the left hand side and right hand side
                if lhs_type != MirType::void().refer() && lhs_type != rhs_type.refer() {
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

                // Check if the condition is a structure or of type `void`
                if cond.get_type(vars, funcs, structs)?.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::While(cond, body) => {
                cond.type_check(vars, funcs, structs)?;

                // Check if the condition is a structure or of type `void`
                if cond.get_type(vars, funcs, structs)?.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::If(cond, body) => {
                cond.type_check(vars, funcs, structs)?;

                // Check if the condition is a structure or of type `void`
                if cond.get_type(vars, funcs, structs)?.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberCondition(cond.clone()));
                }

                for stmt in body {
                    stmt.type_check(vars, funcs, structs)?
                }
            }

            Self::IfElse(cond, then_body, else_body) => {
                cond.type_check(vars, funcs, structs)?;

                // Check if the condition is a structure or of type `void`
                if cond.get_type(vars, funcs, structs)?.get_size(structs)? != 1 {
                    return Err(MirError::NonNumberCondition(cond.clone()));
                }

                for stmt in then_body {
                    stmt.type_check(vars, funcs, structs)?
                }
                for stmt in else_body {
                    stmt.type_check(vars, funcs, structs)?
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

            Self::Expression(expr) => expr.type_check(vars, funcs, structs)?,
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

            /// Assign an expression to a defined variable
            Self::AssignVariable(var_name, expr) => {
                // Check to see if the variable has been defined
                if let Some(t) = vars.get(var_name) {
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

    String(StringLiteral),
    Float(f64),
    Character(char),
    Void,

    Variable(Identifier),
    Refer(Identifier),
    Deref(Box<Self>),

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
            // Typecheck binary operations
            // Currently, type checking only fails if either the left hand side
            // or the right hand side are of type `void`, or a user defined structure
            Self::Add(lhs, rhs)
            | Self::Subtract(lhs, rhs)
            | Self::Multiply(lhs, rhs)
            | Self::Divide(lhs, rhs) => {
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
            | Self::Void => {}
        }
        Ok(())
    }

    fn assemble(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirFunction>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmStatement>, MirError> {
        Ok(match self {
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
                let mut args = args.clone();
                args.reverse();
                for arg in args {
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
                let mut args = args.clone();
                args.reverse();
                for arg in args {
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
            /// Arithmetic returns the type of the left hand side
            Self::Add(l, _) | Self::Subtract(l, _) | Self::Multiply(l, _) | Self::Divide(l, _) => {
                l.get_type(vars, funcs, structs)?
            }
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
            Self::Add(lhs, rhs) => write!(f, "{}+{}", lhs, rhs),
            Self::Subtract(lhs, rhs) => write!(f, "{}-{}", lhs, rhs),
            Self::Multiply(lhs, rhs) => write!(f, "{}/{}", lhs, rhs),
            Self::Divide(lhs, rhs) => write!(f, "{}/{}", lhs, rhs),

            Self::Alloc(size) => write!(f, "alloc({})", size),

            Self::Void => write!(f, "@"),
            Self::Character(ch) => write!(f, "{}", ch),
            Self::Float(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{:?}", s),

            Self::Index(ptr, idx) => write!(f, "{}[{}]", ptr, idx),
            Self::Method(expr, method, args) => {
                write!(f, "{}.{}(", expr, method)?;
                for arg in args {
                    write!(f, "{},", arg)?;
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
