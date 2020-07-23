use std::collections::BTreeMap;

use crate::{
    asm::{AsmExpression, AsmFunction, AsmProgram, AsmStatement, AsmType},
    Identifier, StringLiteral,
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MirError {
    FunctionNotDefined(Identifier),
    VariableNotDefined(Identifier),
    MethodNotDefined(MirType, Identifier),
    StructureNotDefined(Identifier),
    DereferenceNonPointer(MirType),
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct MirType {
    name: Identifier,
    ptr_level: i32,
}

impl MirType {
    const FLOAT: &'static str = "float";
    const CHAR: &'static str = "char";
    const VOID: &'static str = "void";

    pub fn structure(name: Identifier) -> Self {
        Self { name, ptr_level: 0 }
    }

    pub fn float() -> Self {
        Self::structure(Identifier::from(Self::FLOAT))
    }

    pub fn character() -> Self {
        Self::structure(Identifier::from(Self::CHAR))
    }

    pub fn void() -> Self {
        Self::structure(Identifier::from(Self::VOID))
    }

    pub fn is_pointer(&self) -> bool {
        self.ptr_level > 0
    }

    pub fn to_asm_type(
        &self,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<AsmType, MirError> {
        let mut result = AsmType::new(self.get_size(structs)?);
        for _ in 0..self.ptr_level {
            result = result.refer();
        }
        Ok(result)
    }

    pub fn get_size(&self, structs: &BTreeMap<Identifier, MirStructure>) -> Result<i32, MirError> {
        Ok(match self.name.as_str() {
            "void" => 0,
            "float" => 1,
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

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct MirProgram(Vec<MirDeclaration>, i32);

impl MirProgram {
    pub fn new(decls: Vec<MirDeclaration>, heap_size: i32) -> Self {
        Self(decls, heap_size)
    }

    pub fn assemble(&self) -> Result<AsmProgram, MirError> {
        let Self(decls, heap_size) = self.clone();
        let mut funcs = BTreeMap::new();
        let mut structs = BTreeMap::new();
        let mut result = Vec::new();
        for decl in &decls {
            match decl {
                MirDeclaration::Function(func) => {
                    funcs.insert(func.get_name(), func.get_return_type());
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
        funcs: &mut BTreeMap<Identifier, MirType>,
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
        funcs: &mut BTreeMap<Identifier, MirType>,
        structs: &BTreeMap<Identifier, MirStructure>,
    ) -> Result<Vec<AsmFunction>, MirError> {
        let mir_type = self.to_mir_type();
        let mut result = Vec::new();
        // Iterate over the methods and rename them to their method names.
        for function in &self.methods {
            let method = function.as_method(&mir_type);
            funcs.insert(
                method.get_name(),
                method.get_return_type(),
            );
            result.push(method.assemble(funcs, structs)?);
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
        funcs: &BTreeMap<Identifier, MirType>,
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
            asm_body.extend(stmt.assemble(&mut vars, funcs, structs)?)
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

    Free(MirExpression, MirExpression),
    Expression(MirExpression),
}

impl MirStatement {
    fn assemble(
        &self,
        vars: &mut BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirType>,
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
                let mut asm_body = Vec::new();
                for stmt in body {
                    asm_body.extend(stmt.assemble(vars, funcs, structs)?);
                }
                vec![AsmStatement::For(
                    pre.assemble(vars, funcs, structs)?,
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
    fn assemble(
        &self,
        vars: &BTreeMap<Identifier, MirType>,
        funcs: &BTreeMap<Identifier, MirType>,
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
                result.push(AsmStatement::Expression(vec![AsmExpression::Deref(
                    expr.get_type(vars, funcs, structs)?.get_size(structs)?,
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
        funcs: &BTreeMap<Identifier, MirType>,
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
            Self::Alloc(_) => MirType::character().refer(),

            /// Get the type of the instance, retrieve the method from the type,
            /// then get the return type of the method.
            Self::Method(expr, method_name, _) => {
                // Get the type of the object
                let instance_type = expr.get_type(vars, funcs, structs)?;
                // Get the return type of the method
                let func_name = instance_type.method_to_function_name(method_name);
                if let Some(func_type) = funcs.get(&func_name) {
                    func_type.clone()
                } else {
                    return Err(MirError::MethodNotDefined(instance_type, func_name.clone()));
                }
            }

            /// When a pointer is indexed, the resulting type is
            /// a pointer of the same type. This is because indexing
            /// a pointer returns the address of the object in the array.
            Self::Index(ptr, _) => ptr.get_type(vars, funcs, structs)?,

            /// Get the return type of the called function
            Self::Call(func_name, _) => {
                if let Some(t) = funcs.get(func_name) {
                    t.clone()
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
