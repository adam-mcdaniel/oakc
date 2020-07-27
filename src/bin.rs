use oakc::{compile, Go, C, TS};
use std::{fs::read_to_string, path::PathBuf, process::exit};

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
			(@arg ts: --ts "Compile with TypeScript backend")
        )
    )
    .setting(ArgRequiredElseHelp)
    .get_matches();

    if let Some(input_file) = matches.value_of("FILE") {
        if let Ok(mut contents) = read_to_string(input_file) {
            let mut result = String::new();
            if let Some(ffi_files) = matches.values_of("FFI") {
                for ffi_file in ffi_files {
                    if let Ok(ffi_contents) = read_to_string(ffi_file) {
                        result += &ffi_contents;
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

            let success = if matches.is_present("c") {
                compile(&cwd, contents, C)
            } else if matches.is_present("go") {
				compile(&cwd, contents, Go)
			} else if matches.is_present("ts") {
				compile(&cwd, contents, TS)
            } else {
                compile(&cwd, contents, C)
            };

            if success {
                println!("compilation was successful");
            } else {
                eprintln!("error: failed to compile generated output code");
            }
        } else {
            eprintln!("error: input file \"{}\" doesn't exist", input_file);
        }
    } else {
        eprintln!("error: no input file given");
    }
}
