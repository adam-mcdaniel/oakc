use super::Target;
use std::{
    env::consts::EXE_SUFFIX,
    fs::{remove_file, write},
    io::{Error, ErrorKind, Result, Write},
    process::{Command, Stdio},
};

pub struct C;
impl Target for C {
    fn get_name(&self) -> char { 'c' }

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

    fn sign(&self) -> String {
        String::from("machine_sign(vm);\n")
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

    fn compile(&self, code: String) -> Result<()> {
        let mut child = Command::new("gcc")
            .arg("-O2")
            .args(&["-o", &format!("main{}", EXE_SUFFIX)[..]])
            .args(&["-x", "c", "-"])
            .stdin(Stdio::piped())
            .spawn();

        if let Ok(mut child) = child {
            match child.stdin.as_mut() {
                Some(stdin) => {
                    if let Err(error) = stdin.write_all(code.as_bytes()) {
                        return Result::Err(Error::new(
                            ErrorKind::Other,
                            "unable to open write to child stdin",
                        ));
                    }
                }
                None => {
                    return Result::Err(Error::new(ErrorKind::Other, "unable to open child stdin"))
                }
            }

            match child.wait_with_output() {
                Ok(_) => return Result::Ok(()),
                Err(_) => {
                    return Result::Err(Error::new(ErrorKind::Other, "unable to read child output"))
                }
            }
        } else {
            // child failed to execute
            Result::Err(Error::new(
                ErrorKind::Other,
                "unable to spawn child gcc proccess",
            ))
        }
    }
}
