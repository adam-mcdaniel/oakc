use termimad::*;
use clap::{clap_app, crate_authors, crate_version, AppSettings::ArgRequiredElseHelp};
use oakc::{compile, generate_docs, Go, C};
use std::{
    fs::{read_to_string, write},
    io::Result,
    path::PathBuf,
};

fn main() {
    let matches = clap_app!(oak =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Compiler for the Oak programming langauge")
        (@subcommand c =>
            (about: "Compile an Oak file")
            (@arg FILE: +required "The input file to use")
            (@group target =>
                (@arg c: -c "Compile with C backend")
                (@arg go: -g --go "Compile with Golang backend")
            )
        )
        (@subcommand doc =>
            (about: "Generate documentation for an Oak file")
            (@arg FILE: +required "The input file to use")
            (@arg OUTPUT: -o +takes_value "The output file")
        )
    )
    .setting(ArgRequiredElseHelp)
    .get_matches();

    if let Some(matches) = matches.subcommand_matches("c") {
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
    } else if let Some(matches) = matches.subcommand_matches("doc") {
        if let Some(input_file) = matches.value_of("FILE") {
            if let Ok(contents) = read_to_string(input_file) {
                let docs = generate_docs(contents, input_file);
                if let Some(output_file) = matches.value_of("OUTPUT") {
                    if let Ok(_) = write(output_file, docs) {
                        println!("doc generation successful")
                    } else {
                        eprintln!("error: could not write to file \"{}\"", output_file);
                    }
                } else {
                    print_text(&docs)
                }
            } else {
                eprintln!("error: input file \"{}\" doesn't exist", input_file);
            }
        } else {
            eprintln!("error: no input file given");
        }
    }
}
