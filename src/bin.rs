use clap::{clap_app, crate_authors, crate_version, AppSettings::ArgRequiredElseHelp};
use oakc::{compile, generate_docs, Go, C, TS};
use std::{
    fs::{read_to_string, write},
    io::Result,
    path::PathBuf,
};
use termimad::*;

fn main() {
    let matches = clap_app!(oak =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Compiler for the Oak programming langauge")
        (@group target =>
            (@arg cc: -c --cc "Compile with C backend")
            (@arg go: -g --go "Compile with Golang backend")
            (@arg ts: -t --ts "Compile with TypeScript backend")
        )
        (@subcommand c =>
            (about: "Compile an Oak file")
            (@arg FILE: +required "The input file to use")
        )
        (@subcommand doc =>
            (about: "Generate documentation for an Oak file")
            (@arg FILE: +required "The input file to use")
            (@arg OUTPUT: -o +takes_value "The output file")
        )
    )
    .setting(ArgRequiredElseHelp)
    .get_matches();

    // If the compile subcommand is being used
    if let Some(sub_matches) = matches.subcommand_matches("c") {
        // Get the input file
        if let Some(input_file) = sub_matches.value_of("FILE") {
            // Get the contents of the input file
            if let Ok(contents) = read_to_string(input_file) {
                // Get the current working directory of the input file
                let cwd = if let Some(dir) = PathBuf::from(input_file).parent() {
                    PathBuf::from(dir)
                } else {
                    PathBuf::from("./")
                };

                // Compile using the target backend
                let compile_result = if matches.is_present("cc") {
                    compile(&cwd, &input_file, contents, C)
                } else if matches.is_present("go") {
                    compile(&cwd, &input_file, contents, Go)
                } else if matches.is_present("ts") {
                    compile(&cwd, &input_file, contents, TS)
                } else {
                    compile(&cwd, &input_file, contents, C)
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
    // If the documentation subcommand is being used
    } else if let Some(sub_matches) = matches.subcommand_matches("doc") {
        // Get the input file
        if let Some(input_file) = sub_matches.value_of("FILE") {
            // Get the contents of the input file
            if let Ok(contents) = read_to_string(input_file) {
                // Get the current working directory of the input file
                let cwd = if let Some(dir) = PathBuf::from(input_file).parent() {
                    PathBuf::from(dir)
                } else {
                    PathBuf::from("./")
                };

                // Document the input file using the target backend
                let docs = if matches.is_present("cc") {
                    generate_docs(&cwd, input_file, contents, C)
                } else if matches.is_present("go") {
                    generate_docs(&cwd, input_file, contents, Go)
                } else {
                    generate_docs(&cwd, input_file, contents, C)
                };

                // If the output file exists, write the output to it
                if let Some(output_file) = sub_matches.value_of("OUTPUT") {
                    if write(output_file, docs).is_ok() {
                        println!("doc generation successful")
                    } else {
                        eprintln!("error: could not write to file \"{}\"", output_file);
                    }
                } else {
                    // If no output file is specified, pretty print the markdown
                    println!("{}", make_skin().term_text(&docs));
                }
            } else {
                eprintln!("error: input file \"{}\" doesn't exist", input_file);
            }
        } else {
            eprintln!("error: no input file given");
        }
    }
}

/// Get the theme for printing the documentation
/// markdown to the terminal.
fn make_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    // Pink
    skin.bold.set_fg(rgb(80, 250, 123));
    // Green
    skin.italic.set_fg(rgb(255, 121, 198));
    // Cyan
    skin.bullet = StyledChar::from_fg_char(rgb(139, 233, 253), '»');
    skin.code_block.align = Alignment::Center;
    skin
}
