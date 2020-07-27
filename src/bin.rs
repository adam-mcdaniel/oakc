use oakc::{compile, Go, C};
use std::{fs::read_to_string, io::Result, path::PathBuf, process::exit};

use clap::{clap_app, crate_authors, crate_version, AppSettings::ArgRequiredElseHelp};

fn main() {
    let matches = clap_app!(oak =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Compiler for the Oak programming langauge")
        (@arg FILE: +required "The input file to use")
        (@arg DEBUG: -d "Enable debugging")
        (@arg FFI: -f --ffi +takes_value ... "Files to use for FFI")
        (@group target =>
            (@arg c: -c "Compile with C backend")
            (@arg go: -g --go "Compile with Golang backend")
        )
    )
    .setting(ArgRequiredElseHelp)
    .get_matches();

    if let Some(input_file) = matches.value_of("FILE") {
        if let Ok(mut contents) = read_to_string(input_file) {
            let mut ffi_code = String::new();
            if let Some(ffi_files) = matches.values_of("FFI") {
                // For each FFI file, add its code to the compiled output
                for ffi_file in ffi_files {
                    // Get the contents of the FFI file
                    if let Ok(ffi_contents) = read_to_string(ffi_file) {
                        // Add the FFI file's contents to the ffi code
                        // and add a newline at the end for good measure
                        ffi_code += &(ffi_contents + "\n");
                    } else {
                        eprintln!("error: FFI file \"{}\" doesn't exist", input_file);
                        exit(1);
                    }
                }
            }

            contents += include_str!("std.ok");

            let cwd = if let Some(dir) = PathBuf::from(input_file).parent() {
                PathBuf::from(dir)
            } else {
                PathBuf::from("./")
            };

            let compile_result = if matches.is_present("c") {
                compile(&cwd, ffi_code, contents, C)
            } else if matches.is_present("go") {
                compile(&cwd, ffi_code, contents, Go)
            } else {
                compile(&cwd, ffi_code, contents, C)
            };

            match compile_result {
                Result::Ok(_) => println!("compilation successful"),
                Result::Err(error) => {
                    if let Some(inner_error) = error.get_ref() {
                        eprintln!("error: {}", inner_error);
                    }
                }
            }
        } else {
            eprintln!("error: input file \"{}\" doesn't exist", input_file);
        }
    } else {
        eprintln!("error: no input file given");
    }
}
