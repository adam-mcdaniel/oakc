use crate::{
    target::{Target, C},
    Identifier, StringLiteral,
};
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display, Error, Formatter},
    fs::read_to_string,
    path::PathBuf,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum AsmError {
    NonExistantExternFile(String),
    VariableNotDefined(Identifier),
    FunctionNotDefined(Identifier),
    NoEntryPoint,
}

impl Display for AsmError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::NonExistantExternFile(filename) => {
                write!(f, "could not find foreign file '{}'", filename)
            }
            Self::FunctionNotDefined(name) => write!(f, "function '{}' is not defined", name),
            Self::VariableNotDefined(name) => write!(f, "variable '{}' is not defined", name),
            Self::NoEntryPoint => write!(f, "no entry point defined"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsmType {
    ptr_level: i32,
    size: i32,
}

impl AsmType {
    pub fn new(size: i32) -> Self {
        Self { ptr_level: 0, size }
    }

    pub fn ch() -> Self {
        Self::new(1)
    }
    pub fn float() -> Self {
        Self::new(1)
    }
    pub fn void() -> Self {
        Self::new(0)
    }

    pub fn refer(&self) -> Self {
        let mut copy = *self;
        copy.ptr_level += 1;
        copy
    }

    pub fn deref(&self) -> Option<Self> {
        if self.ptr_level > 0 {
            let mut copy = *self;
            copy.ptr_level += 1;
            Some(copy)
        } else {
            None
        }
    }

    pub fn get_size(&self) -> i32 {
        if self.ptr_level > 0 {
            1
        } else {
            self.size
        }
    }
}

impl Debug for AsmType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for _ in 0..self.ptr_level {
            write!(f, "&")?;
        }
        write!(f, "{}", self.size)
    }
}

#[derive(Clone, Debug)]
pub struct AsmProgram {
    externs: Vec<PathBuf>,
    funcs: Vec<AsmFunction>,
    heap_size: i32,
}

impl AsmProgram {
    const ENTRY_POINT: &'static str = "main";

    pub fn new(externs: Vec<PathBuf>, funcs: Vec<AsmFunction>, heap_size: i32) -> Self {
        Self {
            externs,
            funcs,
            heap_size,
        }
    }

    pub fn assemble(&self, target: &impl Target) -> Result<String, AsmError> {
        // Set up the output code
        let mut result = String::new();

        // Iterate over the external files to include
        for filename in &self.externs {
            // Find them in the current working directory
            if let Ok(contents) = read_to_string(filename.clone()) {
                // Add the contents of the file to the result
                result += &contents
            } else {
                // If the file doesn't exist, throw an error
                if let Ok(name) = filename.clone().into_os_string().into_string() {
                    return Err(AsmError::NonExistantExternFile(name));
                } else {
                    return Err(AsmError::NonExistantExternFile(String::from("")));
                }
            }
        }

        // Store the IDs of each function
        let mut func_ids = BTreeMap::new();
        // The number of cells to preemptively allocate on the stack
        let mut var_size = 0;
        for (id, func) in self.funcs.iter().enumerate() {
            // Store the function's ID
            func_ids.insert(func.name.clone(), id as i32);
            // Add the function header to the output code
            result += &target.fn_header(AsmFunction::get_assembled_name(id as i32));
        }

        // It is very important that the entry point is assembled last.
        // This is because of the way things are allocated on the stack.
        let mut entry_point = None;
        for func in &self.funcs {
            // Compile the function
            if !func.is_entry_point() {
                result += &func.assemble(&func_ids, &mut var_size, target)?;
            } else {
                // Store the entry point for use later
                // This has the side effect of ignoring multiple definitions
                // of the `main` function, and just using the last one defined.
                entry_point = Some(func);
            }
        }

        if let Some(func) = entry_point {
            if let Some(main_id) = func_ids.get(Self::ENTRY_POINT) {
                // Assemble the entry point code
                result += &func.assemble(&func_ids, &mut var_size, target)?;

                // Call the entry point
                result += &target.begin_entry_point(var_size, self.heap_size);
                result += &target.call_fn(AsmFunction::get_assembled_name(*main_id));
                result += &target.end_entry_point();

                // FOR DEBUGGING
                // println!("STACK SIZE: {}", var_size);

                Ok(result)
            } else {
                Err(AsmError::NoEntryPoint)
            }
        } else {
            Err(AsmError::NoEntryPoint)
        }
    }
}

#[derive(Clone, Debug)]
pub struct AsmFunction {
    name: Identifier,
    args: Vec<(Identifier, AsmType)>,
    return_type: AsmType,
    body: Vec<AsmStatement>,
}

impl AsmFunction {
    pub fn new(
        name: Identifier,
        args: Vec<(Identifier, AsmType)>,
        return_type: AsmType,
        body: Vec<AsmStatement>,
    ) -> Self {
        Self {
            name,
            args,
            return_type,
            body,
        }
    }

    fn is_entry_point(&self) -> bool {
        self.name == AsmProgram::ENTRY_POINT
    }

    /// Use the function's ID to get the output code's name of the function.
    /// An ID is used to prevent invalid output code function names, or names
    /// that clash with standard library names such as "printf" or "malloc".
    fn get_assembled_name(id: i32) -> String {
        format!("fn{}", id)
    }

    fn assemble(
        &self,
        func_ids: &BTreeMap<String, i32>,
        var_size: &mut i32,
        target: &impl Target,
    ) -> Result<String, AsmError> {
        let mut result = String::new();

        // Store the variables's addresses and types in the scope
        let mut vars = BTreeMap::new();
        for (arg_name, arg_type) in &self.args {
            // Define each argument of the function
            result += &AsmStatement::Define(arg_name.clone(), *arg_type)
                .assemble(func_ids, &mut vars, var_size, target)?;
            result +=
                &AsmStatement::Assign(*arg_type).assemble(func_ids, &mut vars, var_size, target)?;
        }

        for stmt in &self.body {
            // Assemble each statement in the function body
            result += &stmt.assemble(func_ids, &mut vars, var_size, target)?;
        }

        // Write the function as output code
        if let Some(id) = func_ids.get(&self.name) {
            Ok(target.fn_definition(Self::get_assembled_name(*id), result))
        } else {
            Err(AsmError::FunctionNotDefined(self.name.clone()))
        }
    }
}

#[derive(Clone, Debug)]
pub enum AsmStatement {
    For(Vec<Self>, Vec<Self>, Vec<Self>, Vec<Self>),
    Define(Identifier, AsmType),
    Assign(AsmType),
    Expression(Vec<AsmExpression>),
}

impl AsmStatement {
    fn assemble(
        &self,
        func_ids: &BTreeMap<String, i32>,
        vars: &mut BTreeMap<String, (i32, AsmType)>,
        var_size: &mut i32,
        target: &impl Target,
    ) -> Result<String, AsmError> {
        Ok(match self {
            // Define a variable on the stack
            Self::Define(name, data_type) => {
                let address = *var_size;
                // Add the variable's location and type to the scope
                vars.insert(name.clone(), (address, *data_type));
                // Increment the size of the program's variables
                *var_size += data_type.get_size();
                // Push the address of the new variable onto the stack
                target.push(address as f64)
            }
            // Pop an address off of the stack, pop an item of size `data_type`
            // off of the stack, and store the item at the address
            Self::Assign(data_type) => target.store(data_type.get_size()),
            Self::For(pre, cond, post, body) => {
                let mut result = String::new();
                // Run the code that preps the for loop
                for stmt in pre {
                    result += &stmt.assemble(func_ids, vars, var_size, target)?;
                }
                // Check the condition of the for loop
                for expr in cond {
                    result += &expr.assemble(func_ids, vars, var_size, target)?;
                }
                // Begin the loop body
                result += &target.begin_while();
                // Run the body of the loop
                for stmt in body {
                    result += &stmt.assemble(func_ids, vars, var_size, target)?;
                }
                // Run the code that procedes the body of the loop
                for stmt in post {
                    result += &stmt.assemble(func_ids, vars, var_size, target)?;
                }
                // Check the condition again
                for expr in cond {
                    result += &expr.assemble(func_ids, vars, var_size, target)?;
                }
                // End the loop body
                result + &target.end_while()
            }

            Self::Expression(exprs) => {
                let mut result = String::new();
                for expr in exprs {
                    result += &expr.assemble(func_ids, vars, var_size, target)?;
                }
                result
            }
        })
    }
}

#[derive(Clone, Debug)]
pub enum AsmExpression {
    String(StringLiteral),
    Character(char),
    Float(f64),
    Void,

    ForeignCall(Identifier),

    Variable(Identifier),
    Call(Identifier),
    Refer(Identifier),
    Deref(i32),

    Alloc,
    Free,

    Divide,
    Multiply,
    Subtract,
    Add,
    Sign,
}

impl AsmExpression {
    fn assemble(
        &self,
        func_ids: &BTreeMap<String, i32>,
        vars: &BTreeMap<String, (i32, AsmType)>,
        var_size: &mut i32,
        target: &impl Target,
    ) -> Result<String, AsmError> {
        Ok(match self {
            Self::String(s) => {
                // The address of the string is at the current first
                // empty spot on the stack.
                let address = *var_size;
                // The size of the string is the length of the characters,
                // plus 1 for the zero terminated character.
                let size = s.len() as i32 + 1;

                // Push each character of the string onto the stack
                let mut result = String::new();
                for ch in s.chars() {
                    result += &target.push(ch as u8 as f64);
                }
                // Push the zero terminated character
                result += &target.push(0.0);

                // Store the characters at the address of the string,
                // and push the address onto the stack.
                result += &(target.push(address as f64)
                    + &target.store(size)
                    + &target.push(address as f64));

                // Increment the amount of data stored on the stack
                *var_size += size;
                result
            }
            // Push a character onto the stack
            Self::Character(ch) => target.push(*ch as u8 as f64),
            // Push a float onto the stack
            Self::Float(n) => target.push(*n),
            // Void expressions are a No-Op
            Self::Void => String::new(),

            // Load a variable onto the stack with a given type
            Self::Variable(name) => {
                // Get the address of the variable on the stack
                // and the type of the variable
                if let Some((addr, data_type)) = vars.get(name) {
                    // Push the address and load the data at that address
                    target.push(*addr as f64) + &target.load(data_type.get_size())
                } else {
                    return Err(AsmError::VariableNotDefined(name.clone()));
                }
            }

            // Call a function
            Self::Call(fn_name) => {
                if let Some(fn_id) = func_ids.get(fn_name) {
                    target.call_fn(AsmFunction::get_assembled_name(*fn_id))
                } else {
                    return Err(AsmError::FunctionNotDefined(fn_name.clone()));
                }
            }

            // Call a foreign function
            Self::ForeignCall(fn_name) => target.call_foreign_fn(fn_name.clone()),

            // Allocate data on the heap
            Self::Alloc => target.allocate(),
            // Free data on the heap
            Self::Free => target.free(),
            // Get the address of a variable on the stack
            Self::Refer(name) => {
                if let Some((addr, _)) = vars.get(name) {
                    target.push(*addr as f64)
                } else {
                    return Err(AsmError::VariableNotDefined(name.clone()));
                }
            }
            // Dereference an address
            Self::Deref(size) => target.load(*size),

            // Get the absolute value of a number on the stack
            Self::Sign => target.sign(),
            // Add two numbers on the stack
            Self::Add => target.add(),
            // Subtract two numbers on the stack
            Self::Subtract => target.subtract(),
            // Multiply two numbers on the stack
            Self::Multiply => target.multiply(),
            // Divide two numbers on the stack
            Self::Divide => target.divide(),
        })
    }
}
