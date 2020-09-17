use super::Target;
use std::{
    fs::{remove_file, write},
    io::{Error, ErrorKind, Result},
    process::Command,
};

pub struct Go;
impl Target for Go {
    fn get_name(&self) -> char {
        'g'
    }

    fn is_standard(&self) -> bool {
        true
    }

    fn std(&self) -> String {
        String::from(include_str!("std/std.go"))
    }

    fn core_prelude(&self) -> String {
        String::from(include_str!("core/core.go"))
    }

    fn core_postlude(&self) -> String {
        String::new()
    }

    fn begin_entry_point(&self, global_scope_size: i32, memory_size: i32) -> String {
        format!(
            "func main() {{\nvm := machine_new({}, {})\n",
            global_scope_size,
            global_scope_size + memory_size,
        )
    }

    fn end_entry_point(&self) -> String {
        String::from("\nvm.drop()\n}")
    }

    fn establish_stack_frame(&self, arg_size: i32, local_scope_size: i32) -> String {
        format!(
            "vm.establish_stack_frame({}, {})\n",
            arg_size, local_scope_size
        )
    }

    fn end_stack_frame(&self, return_size: i32, local_scope_size: i32) -> String {
        format!(
            "vm.end_stack_frame({}, {})\n",
            return_size, local_scope_size
        )
    }

    fn load_base_ptr(&self) -> String {
        String::from("vm.load_base_ptr()\n")
    }

    fn push(&self, n: f64) -> String {
        format!("vm.push({})\n", n)
    }

    fn add(&self) -> String {
        String::from("vm.add()\n")
    }

    fn subtract(&self) -> String {
        String::from("vm.subtract()\n")
    }

    fn multiply(&self) -> String {
        String::from("vm.multiply()\n")
    }

    fn divide(&self) -> String {
        String::from("vm.divide()\n")
    }

    fn sign(&self) -> String {
        String::from("vm.sign()\n")
    }

    fn allocate(&self) -> String {
        String::from("vm.allocate()\n")
    }

    fn free(&self) -> String {
        String::from("vm.free()\n")
    }

    fn store(&self, size: i32) -> String {
        format!("vm.store({})\n", size)
    }

    fn load(&self, size: i32) -> String {
        format!("vm.load({})\n", size)
    }

    fn fn_header(&self, name: String) -> String {
        String::new()
    }

    fn fn_definition(&self, name: String, body: String) -> String {
        format!("\n\nfunc {}(vm *machine) {{\n{}\n}}\n", name, body)
    }

    fn call_fn(&self, name: String) -> String {
        format!("{}(vm);\n", name)
    }

    fn call_foreign_fn(&self, name: String) -> String {
        format!("{}(vm);\n", name)
    }

    fn begin_while(&self) -> String {
        String::from("for vm.pop() != 0.0 {\n")
    }

    fn end_while(&self) -> String {
        String::from("}\n")
    }

    fn compile(&self, code: String) -> Result<()> {
        if let Ok(_) = write("main.go", code) {
            if let Ok(_) = Command::new("go").arg("build").arg("main.go").output() {
                if let Ok(_) = remove_file("main.go") {
                    return Result::Ok(());
                }
            }
        }
        Result::Err(Error::new(
            ErrorKind::Other,
            "could not compile output golang code. is golang installed?",
        ))
    }
}
