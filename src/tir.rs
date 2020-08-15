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
    /// A copy method must have a very specific type signature:
    /// fn copy(self: &T) -> T
    /// This is so that the compiler can properly place
    /// copy and drop method calls for automatic memory management.
    InvalidCopyTypeSignature(Identifier),
    /// A drop method must have a very specific type signature:
    /// fn drop(self: &T) -> void
    /// This is so that the compiler can properly place
    /// copy and drop method calls for automatic memory management.
    InvalidDropTypeSignature(Identifier),
    /// Does a structure use a member with an undefined type?
    /// If so, then this error is thrown.
    StructureNotDefined(Identifier),
    /// The user may NOT call the `.copy()` method explicitly
    /// The compiler is only allowed to call this method.
    /// This is to prevent memory leaks.
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

/// This is purely a standin for HIR's declaration
/// type. However, if a `macro` flag is added, it
/// should be added here.
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

/// This enum represents a type name in an expression.
/// Take for example the declaration `fn test(x: num) -> &void`.
/// `num` and `&void` are both `TirType` instances.
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
    /// Is this type a structure?
    fn is_structure(&self) -> bool {
        match self {
            Self::Structure(_) => true,
            _ => false,
        }
    }

    /// Can this type be moved without making a new copy?
    fn is_movable(&self, decls: &Vec<TirDeclaration>) -> Result<bool, TirError> {
        if let Self::Structure(name) = self {
            for decl in decls {
                if let TirDeclaration::Structure(structure) = decl {
                    // Find the structure with this type's name,
                    // and return if it is movable
                    if name == structure.get_name() {
                        return Ok(structure.is_movable(decls)?);
                    }
                }
            }
            // If the structure is not defined, then this type is not defined
            return Err(TirError::StructureNotDefined(name.clone()));
        } else {
            // If this type is not a structure,
            // it is movable.
            return Ok(true);
        }
    }

    /// Add a reference to this type
    fn refer(&self) -> Self {
        Self::Pointer(Box::new(self.clone()))
    }

    /// Convert this type to an HIR type
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

/// The type that represents a function definition.
#[derive(Clone, Debug)]
pub struct TirFunction {
    /// The function's optional docstring
    doc: Option<String>,
    /// The function's name
    name: Identifier,
    /// The function's parameters
    args: Vec<(Identifier, TirType)>,
    /// The function's return type
    return_type: TirType,
    /// The function's body statements
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

    /// A structure in Oak is actually syntactic
    /// sugar for a method. Take for example the
    /// following structure definition:
    /// ```
    /// struct Date {
    ///     let month: num,
    ///         day: num,
    ///         year: num;
    /// }
    /// ```
    /// This structure gets converted to the following HIR structure
    /// ```
    /// struct Date(sizeof(num) + sizeof(num) + sizeof(num)) {
    ///     fn month(self: &Date) -> &num { return self as &num}
    ///     fn day(self: &Date) -> &num { return (self + sizeof(num)) as &num}
    ///     fn year(self: &Date) -> &num { return (self + sizeof(num) + sizeof(num)) as &num}
    /// }
    /// ```
    fn member_method(
        // The type of the structure the method is being defined for
        self_type: &Identifier,
        // The list of members that came before this member. This is
        // to determine the offset of the member in the structure's memory.
        previous_member_types: &Vec<TirType>,
        // The name of this member
        member_name: &Identifier,
        // This member's type
        member_type: &TirType,
    ) -> Self {
        // Add the size of all the previous members to the self pointer
        // to get the address of this member.
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
            // Then, typecast the address of the member as the member's type.
            vec![TirStatement::Return(vec![TirExpression::TypeCast(
                Box::new(fn_return),
                member_type.refer().clone(),
            )])],
        )
    }

    /// Generate a copy constructor for a type.
    fn copy_constructor(members: &Vec<(Identifier, TirType)>, structure: &Identifier) -> Self {
        let struct_t = TirType::Structure(structure.clone());
        let mut result = vec![];

        if members.len() == 1 {
            // If the number of members is one, then
            // the returned value NEEDS to be cast to pass MIR typechecks.
            let member_name = members[0].0.clone();

            // This generates the following code:
            // ```
            // return (*self) as T
            // ```
            result = vec![TirExpression::TypeCast(
                Box::new(TirExpression::Deref(Box::new(TirExpression::Method(
                    Box::new(TirExpression::Variable(Identifier::from("self"))),
                    member_name,
                    vec![],
                )))),
                TirType::Structure(structure.clone()),
            )]
        } else {
            // If the number of members greater than one, then
            // the typechecks will pass without casting any members.

            // This generates the following code:
            // ```
            // return [self->member_1, self->member_2, ...];
            // ```
            for (member, _) in members {
                result.push(TirExpression::Deref(Box::new(TirExpression::Method(
                    Box::new(TirExpression::Variable(Identifier::from("self"))),
                    member.clone(),
                    vec![],
                ))))
            }
        }

        // fn copy(self: &T) -> T { ... }
        Self::new(
            None,
            Identifier::from("copy"),
            vec![(Identifier::from("self"), struct_t.refer())],
            struct_t,
            vec![TirStatement::Return(result)],
        )
    }

    /// Generate a drop destructor for a type
    fn drop_destructor(members: &Vec<(Identifier, TirType)>, structure: &Identifier) -> Self {
        // Convert a structure to its TIR type
        let struct_t = TirType::Structure(structure.clone());
        let mut result = vec![];
        for (member, t) in members {
            // If the type of the member is a structure, call its drop method.
            // If the object is a pointer or is primitive, then the drop method
            // must not be called.
            if t.is_structure() {
                // Generate `self->member.drop();`
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

    /// Is the type signature of this function a valid copy constructor for a given type?
    fn is_valid_copy(&self, structure: &Identifier) -> Result<bool, TirError> {
        // The method name must be `copy`
        if &self.name == "copy" {
            let struct_t = TirType::Structure(structure.clone());
            // If the number of parameters is one,
            // and the parameter type is &T,
            // and the result is T, then the type signature is good!
            if self.args.len() == 1
                && self.args[0].1 == struct_t.refer()
                && self.return_type == struct_t
            {
                return Ok(true);
            } else {
                // Otherwise, throw an error about the copy constructors type signature
                return Err(TirError::InvalidCopyTypeSignature(structure.clone()));
            }
        }
        return Ok(false);
    }

    /// Is the type signature of this function a valid drop destructor for a given type?
    fn is_valid_drop(&self, structure: &Identifier) -> Result<bool, TirError> {
        // The method name must be `drop`
        if &self.name == "drop" {
            let struct_t = TirType::Structure(structure.clone());
            // If the number of parameters is one,
            // and the parameter type is &T,
            // and the result is void, then the type signature is good!
            if self.args.len() == 1
                && self.args[0].1 == struct_t.refer()
                && self.return_type == TirType::Void
            {
                return Ok(true);
            } else {
                // Otherwise, throw an error about the drop destructors type signature
                return Err(TirError::InvalidDropTypeSignature(structure.clone()));
            }
        }
        return Ok(false);
    }

    /// Convert this function into an HIR function
    fn to_hir_fn(&self, decls: &Vec<TirDeclaration>) -> Result<HirFunction, TirError> {
        // Convert the parameter types to HIR types
        let mut args = vec![];
        for (arg, t) in &self.args {
            args.push((arg.clone(), t.to_hir_type()?))
        }

        // Convert the function statements to HIR statements
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

/// The type that represents a structure definition.
#[derive(Clone, Debug)]
pub struct TirStructure {
    /// The optional docstring for the structure
    doc: Option<String>,
    /// The name of the structure
    name: Identifier,
    /// The structure's members
    members: Vec<(Identifier, TirType)>,
    /// The structure's methods
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

    /// Get the name of the structure
    fn get_name(&self) -> &Identifier {
        &self.name
    }

    /// Can this type be moved without making a new copy?
    fn is_movable(&self, decls: &Vec<TirDeclaration>) -> Result<bool, TirError> {
        /// Does this type manually implement copy and drop?
        let mut default_copy = true;
        let mut default_drop = true;
        for method in &self.methods {
            // If the method is a copy constructor, mark `default_copy` as false
            if method.is_valid_copy(&self.name)? {
                default_copy = false;
            }

            // If the method is a drop destructor, mark `default_drop` as false
            if method.is_valid_drop(&self.name)? {
                default_drop = false;
            }
        }

        for (_, t) in &self.members {
            // If any of the structure's members are not movable,
            // then this structure cannot be movable.
            if !t.is_movable(decls)? {
                return Ok(false);
            }
        }
        // If either a `copy` or `drop` is implemented manually,
        // then the object cannot be movable.
        Ok(default_copy && default_drop)
    }

    fn to_hir_struct(&mut self, decls: &Vec<TirDeclaration>) -> Result<HirStructure, TirError> {
        // Check if the structure is movable BEFORE the copy
        // and drop functions are automatically added. If the
        // copy and drop methods are added before the movability is checked,
        // then `is_movable` will automatically be false.
        let is_movable = self.is_movable(decls)?;
        // Add the object's `copy` and `drop` methods.
        self.add_copy_and_drop()?;

        // Create the list of methods for the new HIR structure
        let mut methods = vec![];

        // Store all the previous member's types for each member
        // to create a getter/setter method for each member.
        let mut previous_member_types = vec![];
        
        // Keep track of the size of the structure
        let mut size = HirConstant::Float(0.0);
        
        for (name, t) in &self.members {
            // Add the member function to the list of methods
            methods.push(
                TirFunction::member_method(&self.name, &previous_member_types, name, t)
                    .to_hir_fn(decls)?,
            );
            // Add the size of the member to the size of the structure
            size = HirConstant::Add(
                Box::new(size.clone()),
                Box::new(HirConstant::SizeOf(t.to_hir_type()?)),
            );
            // Add this member's type to the list of
            // previous member's types.
            previous_member_types.push(t.clone())
        }

        // In addition to the member methods,
        // add each of the structures explicit methods
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

    /// Add the default copy and drop methods to this structure
    fn add_copy_and_drop(&mut self) -> Result<(), TirError> {
        // To prevent multiple method definitions,
        // determine whether or not the copy and
        // drop methods have already been defined.
        let mut has_copy = false;
        let mut has_drop = false;
        for method in &self.methods {
            if method.is_valid_copy(&self.name)? {
                has_copy = true;
            } else if method.is_valid_drop(&self.name)? {
                has_drop = true;
            }
        }

        // If the structure does not have a copy method,
        // add a default copy constructor to the list of methods.
        if !has_copy {
            self.methods
                .push(TirFunction::copy_constructor(&self.members, &self.name));
        }

        // If the structure does not have a drop method,
        // add a default drop destructor to the list of methods.
        if !has_drop {
            self.methods
                .push(TirFunction::drop_destructor(&self.members, &self.name));
        }

        Ok(())
    }
}

/// The type that represents a constant TIR  expression.
#[derive(Clone, Debug)]
pub enum TirConstant {
    /// A float constant
    Float(f64),
    /// A character constant
    Character(char),
    /// True constant
    True,
    /// False constant
    False,

    /// Add two constants
    Add(Box<Self>, Box<Self>),
    /// Subtract two constants
    Subtract(Box<Self>, Box<Self>),
    /// Multiply two constants
    Multiply(Box<Self>, Box<Self>),
    /// Divide two constants
    Divide(Box<Self>, Box<Self>),

    /// And two constants
    And(Box<Self>, Box<Self>),
    /// Or two constants
    Or(Box<Self>, Box<Self>),
    /// Not a constant
    Not(Box<Self>),

    /// Compare two constants with the `>` operator
    Greater(Box<Self>, Box<Self>),
    /// Compare two constants with the `<` operator
    Less(Box<Self>, Box<Self>),
    /// Compare two constants with the `>=` operator
    GreaterEqual(Box<Self>, Box<Self>),
    /// Compare two constants with the `<=` operator
    LessEqual(Box<Self>, Box<Self>),
    /// Compare two constants with the `==` operator
    Equal(Box<Self>, Box<Self>),
    /// Compare two constants with the `!=` operator
    NotEqual(Box<Self>, Box<Self>),

    /// Get a constant by its name
    Constant(Identifier),
    /// Is a constant defined?
    IsDefined(String),
    /// Is a type movable?
    IsMovable(TirType),
    /// What's the size of a type?
    SizeOf(TirType),
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
    /// Add to a variable
    AddAssignVariable(Identifier, TirExpression),
    /// Subtract from a variable
    SubtractAssignVariable(Identifier, TirExpression),
    /// Multiply to a variable
    MultiplyAssignVariable(Identifier, TirExpression),
    /// Divide from a variable
    DivideAssignVariable(Identifier, TirExpression),
    /// An assignment to a dereferenced address
    AssignAddress(TirExpression, TirExpression),
    /// Add to the value a pointer points to
    AddAssignAddress(TirExpression, TirExpression),
    /// Subtract from the value a pointer points to
    SubtractAssignAddress(TirExpression, TirExpression),
    /// Multiply the value a pointer points to
    MultiplyAssignAddress(TirExpression, TirExpression),
    /// Divide the value a pointer points to
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
