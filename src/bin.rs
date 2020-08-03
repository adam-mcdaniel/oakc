use oakc::{compile, Go, C};
use std::{fs::read_to_string, io::Result, path::PathBuf};

use clap::{clap_app, crate_authors, crate_version, AppSettings::ArgRequiredElseHelp};

fn main() {
    let matches = clap_app!(oak =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Compiler for the Oak programming langauge")
        (@arg FILE: +required "The input file to use")
        (@arg DEBUG: -d "Enable debugging")
        (@group target =>
            (@arg c: -c "Compile with C backend")
            (@arg go: -g --go "Compile with Golang backend")
        )
    )
    .setting(ArgRequiredElseHelp)
    .get_matches();

    if let Some(input_file) = matches.value_of("FILE") {
        if let Ok(contents) = read_to_string(input_file) {
            let cwd = if let Some(dir) = PathBuf::from(input_file).parent() {
                PathBuf::from(dir)
            } else {
                PathBuf::from("./")
            };

            let compile_result = if matches.is_present("c") {
                compile(&cwd, contents, C)
            } else if matches.is_present("go") {
                compile(&cwd, contents, Go)
            } else {
                compile(&cwd, contents, C)
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
