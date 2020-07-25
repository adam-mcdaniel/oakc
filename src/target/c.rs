use super::Target;
use std::{
    env::consts::EXE_SUFFIX,
    fs::{remove_file, write},
    process::Command,
};

pub struct C;
impl Target for C {
    fn prelude(&self) -> String {
        String::from(include_str!("std.c"))
    }

    fn postlude(&self) -> String {
        String::new()
    }

    fn begin_entry_point(&self, var_size: i32, heap_size: i32) -> String {
        format!(
            "int main() {{\nmachine *vm = machine_new({}, {});\n",
            var_size,
            var_size + heap_size,
        )
    }

    fn end_entry_point(&self) -> String {
        String::from("\nmachine_drop(vm);\nreturn 0;\n}")
    }

    fn push(&self, n: f64) -> String {
        format!("machine_push(vm, {});\n", n)
    }

    fn add(&self) -> String {
        String::from("machine_add(vm);\n")
    }

    fn subtract(&self) -> String {
        String::from("machine_subtract(vm);\n")
    }

    fn multiply(&self) -> String {
        String::from("machine_multiply(vm);\n")
    }

    fn divide(&self) -> String {
        String::from("machine_divide(vm);\n")
    }

    fn allocate(&self) -> String {
        String::from("machine_allocate(vm);\n")
    }

    fn free(&self) -> String {
        String::from("machine_free(vm);\n")
    }

    fn store(&self, size: i32) -> String {
        format!("machine_store(vm, {});\n", size)
    }

    fn load(&self, size: i32) -> String {
        format!("machine_load(vm, {});\n", size)
    }

    fn fn_header(&self, name: String) -> String {
        format!("void {}(machine* vm);\n", name)
    }

    fn fn_definition(&self, name: String, body: String) -> String {
        format!("void {}(machine* vm) {{ {}}}\n", name, body)
    }

    fn call_fn(&self, name: String) -> String {
        format!("{}(vm);\n", name)
    }

    fn call_foreign_fn(&self, name: String) -> String {
        format!("{}(vm);\n", name)
    }

    fn begin_while(&self) -> String {
        String::from("while (machine_pop(vm)) {\n")
    }

    fn end_while(&self) -> String {
        String::from("}\n")
    }

    fn compile(&self, code: String) -> bool {
        if let Ok(_) = write("OUTPUT.c", code) {
            if let Ok(_) = Command::new("gcc")
                .arg("OUTPUT.c")
                .arg("-O2")
                .arg("-o")
                .arg(format!("main{}", EXE_SUFFIX))
                .output()
            {
                if let Ok(_) = remove_file("OUTPUT.c") {
                    return true;
                }
            }
        }
        false
    }
}
