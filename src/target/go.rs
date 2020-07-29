use super::Target;
use std::{
    fs::{remove_file, write},
    io::{Error, ErrorKind, Result},
    process::Command,
};

pub struct Go;
impl Target for Go {
    fn get_name(&self) -> char { 'g' }

    fn prelude(&self) -> String {
        String::from(include_str!("std.go"))
    }

    fn postlude(&self) -> String {
        String::new()
    }

    fn begin_entry_point(&self, var_size: i32, heap_size: i32) -> String {
        format!(
            "func main() {{\nvm := machine_new({}, {})\n",
            var_size,
            var_size + heap_size,
        )
    }

    fn end_entry_point(&self) -> String {
        String::from("\nvm.drop()\n}")
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
        Result::Err(Error::new(ErrorKind::Other, "error compiling "))
    }
}
