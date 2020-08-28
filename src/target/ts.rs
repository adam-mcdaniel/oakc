use super::Target;
use std::{
    env::consts::EXE_SUFFIX,
    fs::{remove_file, write},
    io::{Error, ErrorKind, Result, Write},
    process::{Command, Stdio},
};

pub struct TS;
impl Target for TS {
    fn get_name(&self) -> char {
        't'
    }

    fn is_standard(&self) -> bool {
        true
    }

    fn std(&self) -> String {
        String::from(include_str!("std/std.ts"))
    }

    fn core_prelude(&self) -> String {
        String::from(include_str!("core/core.ts"))
    }

    fn core_postlude(&self) -> String {
        String::new()
    }

    fn begin_entry_point(&self, global_scope_size: i32, memory_size: i32) -> String {
        format!(
            "async function OAKmain():Promise<void> {{\nlet vm = machine_new({}, {});\n",
            global_scope_size,
            global_scope_size + memory_size,
        )
    }

    fn end_entry_point(&self) -> String {
        String::from("\nmachine_drop(vm);\n}\nOAKmain();")
    }

    fn establish_stack_frame(&self, arg_size: i32, local_scope_size: i32) -> String {
        format!(
            "machine_establish_stack_frame(vm, {}, {});\n",
            arg_size, local_scope_size
        )
    }

    fn end_stack_frame(&self, return_size: i32, local_scope_size: i32) -> String {
        format!(
            "machine_end_stack_frame(vm, {}, {});\n",
            return_size, local_scope_size
        )
    }

    fn load_base_ptr(&self) -> String {
        String::from("machine_load_base_ptr(vm);\n")
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
        String::from("")
    }

    fn fn_definition(&self, name: String, body: String) -> String {
        format!("async function {}(vm: machine): void {{ {}}}\n", name, body)
    }

    fn call_fn(&self, name: String) -> String {
        format!("await {}(vm);\n", name)
    }

    fn call_foreign_fn(&self, name: String) -> String {
        format!("await {}(vm);\n", name)
    }

    fn begin_while(&self) -> String {
        String::from("while (machine_pop(vm)) {\n")
    }

    fn end_while(&self) -> String {
        String::from("}\n")
    }

    fn compile(&self, code: String) -> Result<()> {
        if let Ok(_) = write("OUTPUT.ts", code) {
            if let Ok(_) = Command::new("tsc")
                .arg("OUTPUT.ts")
                .arg("--outFile")
                .arg("main.js")
                .arg("--target")
                .arg("ES2017")
                .output()
            {
                if let Ok(_) = remove_file("OUTPUT.ts") {
                    return Result::Ok(());
                }
            }
        }
        Result::Err(Error::new(ErrorKind::Other, "error compiling "))
    }
}
