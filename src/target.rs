pub trait Target {
    fn prelude(&self) -> String;
    fn postlude(&self) -> String;

    fn begin_entry_point(&self, heap_size: i32) -> String;
    fn end_entry_point(&self) -> String;

    fn push(&self, n: f64) -> String;

    fn add(&self) -> String;
    fn subtract(&self) -> String;
    fn multiply(&self) -> String;
    fn divide(&self) -> String;

    fn allocate(&self) -> String;
    fn free(&self) -> String;
    fn store(&self, size: i32) -> String;
    fn load(&self, size: i32) -> String;

    fn fn_header(&self, id: i32) -> String;
    fn fn_definition(&self, name: String, body: String) -> String;
    fn call_fn(&self, name: String) -> String;
    fn call_foreign_fn(&self, name: String) -> String;

    fn begin_while(&self) -> String;
    fn end_while(&self) -> String;
}
// #ifndef OAK_H
// #define OAK_H
// #include <stdbool.h>
// #include <stdio.h>

// typedef struct machine {
//     double* memory;
//     bool*   allocated;
//     int     capacity;
//     int     stack_ptr;
// } machine;

// // Fatal error handler. Always exits program.
// void panic(int code);

// ////////////////////////////////////////////////////////////////////////
// ////////////////////// Constructor and destructor //////////////////////
// ////////////////////////////////////////////////////////////////////////
// // Create new virtual machine
// machine *machine_new(int vars, int capacity);
// // Free the virtual machine's memory. This is called at the end of the program.
// void machine_drop(machine *vm);

// /////////////////////////////////////////////////////////////////////////
// ///////////////////// Stack manipulation operations /////////////////////
// /////////////////////////////////////////////////////////////////////////
// // Push a number onto the stack
// void machine_push(machine *vm, double n);
// // Pop a number from the stack
// double machine_pop(machine *vm);
// // Add the topmost numbers on the stack
// void machine_add(machine *vm);
// // Subtract the topmost number on the stack from the second topmost number on the stack
// void machine_subtract(machine *vm);
// // Multiply the topmost numbers on the stack
// void machine_multiply(machine *vm);
// // Divide the second topmost number on the stack by the topmost number on the stack
// void machine_divide(machine *vm);


// /////////////////////////////////////////////////////////////////////////
// ///////////////////// Pointer and memory operations /////////////////////
// /////////////////////////////////////////////////////////////////////////
// // Pop the `size` parameter off of the stack, and return a pointer to `size` number of free cells.
// int machine_allocate(machine *vm);
// // Pop the `address` and `size` parameters off of the stack, and free the memory at `address` with size `size`.
// void machine_free(machine *vm);
// // Pop an `address` parameter off of the stack, and a `value` parameter with size `size`.
// // Then store the `value` parameter at the memory address `address`.
// void machine_store(machine *vm, int size);
// // Pop an `address` parameter off of the stack, and push the value at `address` with size `size` onto the stack.
// void machine_load(machine *vm, int size);

// void prn(machine *vm);
// void prs(machine *vm);
// void prc(machine *vm);
// void prend(machine *vm);
// void getch(machine *vm);

// #endif

// use crate::{Identifier, StringLiteral};
// use core::fmt::{Debug, Error, Formatter};
// use std::collections::BTreeMap;

// #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
// pub enum AsmError {
//     VariableNotDefined(Identifier),
//     FunctionNotDefined(Identifier),
//     NoEntryPoint,
// }

// #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// pub struct AsmType {
//     ptr_level: i32,
//     size: i32,
// }

// impl AsmType {
//     pub fn new(size: i32) -> Self {
//         Self { ptr_level: 0, size }
//     }

//     pub fn ch() -> Self {
//         Self::new(1)
//     }
//     pub fn float() -> Self {
//         Self::new(1)
//     }
//     pub fn void() -> Self {
//         Self::new(0)
//     }

//     pub fn refer(&self) -> Self {
//         let mut copy = *self;
//         copy.ptr_level += 1;
//         copy
//     }

//     pub fn deref(&self) -> Option<Self> {
//         if self.ptr_level > 0 {
//             let mut copy = *self;
//             copy.ptr_level += 1;
//             Some(copy)
//         } else {
//             None
//         }
//     }

//     pub fn get_size(&self) -> i32 {
//         if self.ptr_level > 0 {
//             1
//         } else {
//             self.size
//         }
//     }
// }

// #[derive(Clone)]
// pub struct AsmProgram(Vec<AsmFunction>, i32);

// impl AsmProgram {
//     pub fn new(funcs: Vec<AsmFunction>, heap_size: i32) -> Self {
//         Self(funcs, heap_size)
//     }

//     pub fn assemble(&self) -> Result<String, AsmError> {
//         let Self(func_list, heap_size) = self;
//         // Set up the output code
//         let mut result = String::from("#include \"oak.h\"\n\n");
//         // Store the IDs of each function
//         let mut func_ids = BTreeMap::new();
//         // The number of cells to preemptively allocate on the stack
//         let mut var_size = 0;
//         for (id, func) in func_list.iter().enumerate() {
//             // Store the function's ID
//             func_ids.insert(func.name.clone(), id as i32);
//             // Add the function header to the output code
//             result += &format!(
//                 "void {}(machine *vm);\n",
//                 AsmFunction::get_assembled_name(id as i32)
//             );
//         }

//         for func in func_list {
//             // Compile the function
//             result += &func.assemble(&func_ids, &mut var_size)?;
//         }

//         if let Some(main_id) = func_ids.get("main") {
//             result += &format!(
//                 "int main() {{\nmachine *vm = machine_new({}, {});\n{}(vm);\nmachine_drop(vm);\nreturn 0;\n}}",
//                 var_size, var_size + heap_size,
//                 AsmFunction::get_assembled_name(*main_id)
//             );
//         }

//         Ok(result)
//     }
// }

// #[derive(Clone)]
// pub struct AsmFunction {
//     name: Identifier,
//     args: Vec<(Identifier, AsmType)>,
//     return_type: AsmType,
//     body: Vec<AsmStatement>,
// }

// impl AsmFunction {
//     pub fn new(
//         name: Identifier,
//         args: Vec<(Identifier, AsmType)>,
//         return_type: AsmType,
//         body: Vec<AsmStatement>,
//     ) -> Self {
//         Self {
//             name,
//             args,
//             return_type,
//             body,
//         }
//     }

//     /// Use the function's ID to get the output code's name of the function.
//     /// An ID is used to prevent invalid output code function names, or names
//     /// that clash with standard library names such as "printf" or "malloc".
//     fn get_assembled_name(id: i32) -> String {
//         format!("fn{}", id)
//     }

//     fn assemble(
//         &self,
//         func_ids: &BTreeMap<String, i32>,
//         var_size: &mut i32,
//     ) -> Result<String, AsmError> {
//         let mut result = String::new();

//         // Store the variables's addresses and types in the scope
//         let mut vars = BTreeMap::new();
//         for (arg_name, arg_type) in &self.args {
//             // Define each argument of the function
//             result += &AsmStatement::Define(arg_name.clone(), *arg_type)
//                 .assemble(func_ids, &mut vars, var_size)?;
//             result += &AsmStatement::Assign(*arg_type).assemble(func_ids, &mut vars, var_size)?;
//         }

//         for stmt in &self.body {
//             // Assemble each statement in the function body
//             result += &stmt.assemble(func_ids, &mut vars, var_size)?;
//         }

//         // Write the function as output code
//         if let Some(id) = func_ids.get(&self.name) {
//             Ok(format!(
//                 "void {}(machine *vm) {{\n{}}}\n\n",
//                 Self::get_assembled_name(*id),
//                 result
//             ))
//         } else {
//             Err(AsmError::FunctionNotDefined(self.name.clone()))
//         }
//     }
// }

// #[derive(Clone)]
// pub enum AsmStatement {
//     For(Vec<Self>, Vec<Self>, Vec<Self>, Vec<Self>),
//     Define(Identifier, AsmType),
//     Assign(AsmType),
//     Expression(Vec<AsmExpression>),
// }

// impl AsmStatement {
//     fn assemble(
//         &self,
//         func_ids: &BTreeMap<String, i32>,
//         vars: &mut BTreeMap<String, (i32, AsmType)>,
//         var_size: &mut i32,
//     ) -> Result<String, AsmError> {
//         Ok(match self {
//             // Define a variable on the stack
//             Self::Define(name, data_type) => {
//                 let address = *var_size;
//                 // Add the variable's location and type to the scope
//                 vars.insert(name.clone(), (address, *data_type));
//                 // Increment the size of the program's variables
//                 *var_size += data_type.get_size();
//                 // Push the address of the new variable onto the stack
//                 format!("machine_push(vm, {});", address)
//             }
//             // Pop an address off of the stack, pop an item of size `data_type`
//             // off of the stack, and store the item at the address
//             Self::Assign(data_type) => format!("machine_store(vm, {});\n", data_type.get_size()),
//             Self::For(pre, cond, post, body) => {
//                 let mut result = String::new();
//                 // Run the code that preps the for loop
//                 for stmt in pre {
//                     result += &stmt.assemble(func_ids, vars, var_size)?;
//                 }
//                 // Check the condition of the for loop
//                 for expr in cond {
//                     result += &expr.assemble(func_ids, vars, var_size)?;
//                 }
//                 // Begin the loop body
//                 result += &format!("while (machine_pop(vm)) {{");
//                 // Run the body of the loop
//                 for stmt in body {
//                     result += &stmt.assemble(func_ids, vars, var_size)?;
//                 }
//                 // Run the code that procedes the body of the loop
//                 for stmt in post {
//                     result += &stmt.assemble(func_ids, vars, var_size)?;
//                 }
//                 // Check the condition again
//                 for expr in cond {
//                     result += &expr.assemble(func_ids, vars, var_size)?;
//                 }
//                 // End the loop body
//                 result + "\n}\n"
//             }

//             Self::Expression(exprs) => {
//                 let mut result = String::new();
//                 for expr in exprs {
//                     result += &expr.assemble(func_ids, vars, var_size)?;
//                 }
//                 result
//             }
//         })
//     }
// }

// #[derive(Clone, Debug)]
// pub enum AsmExpression {
//     String(StringLiteral),
//     Character(char),
//     Float(f64),
//     Void,

//     ForeignCall(Identifier),

//     Variable(Identifier),
//     Call(Identifier),
//     Refer(Identifier),
//     Deref(i32),

//     Alloc,
//     Free,

//     Divide,
//     Multiply,
//     Subtract,
//     Add,
// }

// impl AsmExpression {
//     fn assemble(
//         &self,
//         func_ids: &BTreeMap<String, i32>,
//         vars: &BTreeMap<String, (i32, AsmType)>,
//         var_size: &mut i32,
//     ) -> Result<String, AsmError> {
//         Ok(match self {
//             Self::String(s) => {
//                 // The address of the string is at the current first
//                 // empty spot on the stack.
//                 let address = *var_size;
//                 // The size of the string is the length of the characters,
//                 // plus 1 for the zero terminated character.
//                 let size = s.len() as i32 + 1;

//                 // Push each character of the string onto the stack
//                 let mut result = String::new();
//                 for ch in s.chars() {
//                     result += &format!("machine_push(vm, {});\n", ch as u8);
//                 }
//                 // Push the zero terminated character
//                 result += &format!("machine_push(vm, {});\n", 0);
//                 // Store the characters at the address of the string,
//                 // and push the address onto the stack.
//                 result += &format!(
//                     "machine_push(vm, {addr});\nmachine_store(vm, {});\nmachine_push(vm, {addr});\n",
//                     size, addr=address
//                 );
//                 // Increment the amount of data stored on the stack
//                 *var_size += size;
//                 result
//             }
//             // Push a character onto the stack
//             Self::Character(ch) => format!("machine_push(vm, {});\n", *ch as u8),
//             // Push a float onto the stack
//             Self::Float(n) => format!("machine_push(vm, {});\n", *n),
//             // Void expressions are a No-Op
//             Self::Void => String::new(),

//             // Load a variable onto the stack with a given type
//             Self::Variable(name) => {
//                 // Get the address of the variable on the stack
//                 // and the type of the variable
//                 if let Some((addr, data_type)) = vars.get(name) {
//                     // Push the address and load the data at that address
//                     format!(
//                         "machine_push(vm, {});\nmachine_load(vm, {});\n",
//                         addr,
//                         data_type.get_size()
//                     )
//                 } else {
//                     return Err(AsmError::VariableNotDefined(name.clone()));
//                 }
//             }

//             // Call a function
//             Self::Call(fn_name) => {
//                 if let Some(fn_id) = func_ids.get(fn_name) {
//                     format!("{}(vm);\n", AsmFunction::get_assembled_name(*fn_id))
//                 } else {
//                     return Err(AsmError::FunctionNotDefined(fn_name.clone()));
//                 }
//             }

//             // Call a foreign function
//             Self::ForeignCall(fn_name) => format!("{}(vm);\n", fn_name),

//             // Allocate data on the heap
//             Self::Alloc => String::from("machine_allocate(vm);\n"),
//             // Free data on the heap
//             Self::Free => String::from("machine_free(vm);\n"),
//             // Get the address of a variable on the stack
//             Self::Refer(name) => {
//                 if let Some((addr, _)) = vars.get(name) {
//                     format!("machine_push(vm, {});\n", addr)
//                 } else {
//                     return Err(AsmError::VariableNotDefined(name.clone()));
//                 }
//             }
//             // Dereference an address
//             Self::Deref(size) => format!("machine_load(vm, {});\n", size),

//             // Add two numbers on the stack
//             Self::Add => String::from("machine_add(vm);\n"),
//             // Subtract two numbers on the stack
//             Self::Subtract => String::from("machine_subtract(vm);\n"),
//             // Multiply two numbers on the stack
//             Self::Multiply => String::from("machine_multiply(vm);\n"),
//             // Divide two numbers on the stack
//             Self::Divide => String::from("machine_divide(vm);\n"),
//         })
//     }
// }
