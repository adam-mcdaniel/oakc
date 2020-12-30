#![allow(warnings, clippy, unknown_lints)]
use std::{
    collections::BTreeMap,
    env::consts::{FAMILY, OS},
    fmt::Display,
    io::Result,
    path::PathBuf,
    process::exit,
};
pub type Identifier = String;
pub type StringLiteral = String;

pub mod asm;
pub mod hir;
pub mod mir;
pub mod tir;
use hir::{HirConstant, HirProgram};
use tir::TirProgram;

mod target;
pub use target::{Go, Target, C, TS};

use asciicolor::Colorize;
use comment::cpp::strip;
use time::OffsetDateTime;

use lalrpop_util::{lalrpop_mod, ParseError};
lalrpop_mod!(pub parser);

pub fn get_predefined_constants(target: &impl Target) -> BTreeMap<String, HirConstant> {
    let mut constants = BTreeMap::new();

    constants.insert(
        String::from("ON_WINDOWS"),
        HirConstant::boolean(OS == "windows"),
    );
    constants.insert(
        String::from("ON_MACOS"),
        HirConstant::boolean(OS == "macos"),
    );
    constants.insert(
        String::from("ON_LINUX"),
        HirConstant::boolean(OS == "linux"),
    );

    constants.insert(
        String::from("ON_NIX"),
        HirConstant::boolean(FAMILY == "unix"),
    );
    constants.insert(
        String::from("ON_NON_NIX"),
        HirConstant::boolean(FAMILY != "unix"),
    );

    constants.insert(
        String::from("DATE_DAY"),
        HirConstant::Float(OffsetDateTime::now_local().day() as f64),
    );
    constants.insert(
        String::from("DATE_MONTH"),
        HirConstant::Float(OffsetDateTime::now_local().month() as f64),
    );
    constants.insert(
        String::from("DATE_YEAR"),
        HirConstant::Float(OffsetDateTime::now_local().year() as f64),
    );

    constants.insert(
        String::from("TARGET"),
        HirConstant::Character(target.get_name()),
    );
    constants.insert(
        String::from("IS_STANDARD"),
        HirConstant::boolean(target.is_standard()),
    );

    constants
}

pub fn generate_docs(
    // The working directory of the input file.
    // This is where included files will be gathered from.
    cwd: &PathBuf,
    // The name of the input file to generate docs for
    filename: &str,
    // The code to generate docs for
    input: impl ToString,
    // The target to use for the documented code's TARGET const
    target: impl Target,
) -> String {
    match parse(filename, input).compile(cwd, &mut get_predefined_constants(&target)) {
        Ok(output) => output,
        Err(e) => print_compile_error(e),
    }
    .generate_docs(
        filename.to_string(),
        &mut get_predefined_constants(&target),
        false,
    )
}

fn print_compile_error(e: impl Display) -> ! {
    eprintln!("compilation error: {}", e.bright_red().underline());
    exit(1);
}

pub fn compile(
    // The working directory of the input file.
    // This is where included files will be gathered from.
    cwd: &PathBuf,
    // The name of the input file being compiled.
    // This is used for the `current_file()` operator
    filename: &str,
    // The code to compile
    input: impl ToString,
    // The target to compile for
    target: impl Target,
) -> Result<()> {
    let mut constants = get_predefined_constants(&target);

    // Get the TIR code for the user's Oak code
    let mut tir = parse(filename, input);
    // Convert the TIR to HIR
    let mut hir = match tir.compile(cwd, &mut constants) {
        Ok(output) => output,
        Err(e) => print_compile_error(e),
    };

    // Add the core library code to the users code
    hir.extend_declarations(
        match parse("core.ok", include_str!("core.ok"))
            .compile(cwd, &mut constants)
        {
            Ok(output) => output,
            Err(e) => print_compile_error(e),
        }
        .get_declarations(),
    );

    // If the user specifies that they want to include the standard library
    if hir.use_std() {
        // Then add the standard library code to the users code
        hir.extend_declarations(
            match parse("std.ok", include_str!("std.ok"))
                .compile(cwd, &mut constants)
            {
                Ok(output) => output,
                Err(e) => print_compile_error(e),
            }
            .get_declarations(),
        );
    }

    match hir.compile(cwd, &mut constants) {
        Ok(mir) => match mir.assemble() {
            Ok(asm) => match asm.assemble(&target) {
                Ok(result) => target.compile(if hir.use_std() {
                    target.core_prelude() + &target.std() + &result + &target.core_postlude()
                } else {
                    target.core_prelude() + &result + &target.core_postlude()
                }),
                Err(e) => print_compile_error(e),
            },
            Err(e) => print_compile_error(e),
        },
        Err(e) => print_compile_error(e),
    }
}

pub fn parse(filename: &str, input: impl ToString) -> TirProgram {
    // Strip the user's code of all comments
    let code = &strip(input.to_string()).unwrap();

    // Parse the users code and return the resulting TIR
    match parser::ProgramParser::new().parse(filename, &code, code) {
        // if the parser succeeds, build will succeed
        Ok(parsed) => parsed,
        // if the parser succeeds, annotate code with comments
        Err(e) => {
            eprintln!("{}", format_error(&code, e));
            exit(1);
        }
    }
}

type Error<'a, T> = ParseError<usize, T, &'a str>;

/// This formats an error properly given the line, the `unexpected` token as a string,
/// the line number, and the column number of the unexpected token.
fn make_error(line: &str, unexpected: &str, line_number: usize, column_number: usize) -> String {
    // The string used to underline the unexpected token
    let underline = format!(
        "{}^{}",
        " ".repeat(column_number),
        "-".repeat(unexpected.len() - 1)
    );

    // Format string properly and return
    format!(
        "{WS} |
{line_number} | {line}
{WS} | {underline}
{WS} |
{WS} = unexpected `{unexpected}`",
        WS = " ".repeat(line_number.to_string().len()),
        line_number = line_number,
        line = line.bright_yellow().underline(),
        underline = underline,
        unexpected = unexpected.bright_yellow().underline()
    )
}

// Gets the line number, the line, and the column number of the error
pub fn get_line(script: &str, location: usize) -> (usize, String, usize) {
    // Get the line number from the character location
    let line_number = script[..location + 1].lines().count();
    // Get the line from the line number
    let line = match script.lines().nth(line_number - 1) {
        Some(line) => line,
        None => {
            if let Some(line) = script.lines().last() {
                line
            } else {
                ""
            }
        }
    }
    .replace("\t", "    ");

    // Get the column number from the location
    let mut column = {
        let mut current_column = 0;
        // For every character in the script until the location of the error,
        // keep track of the column location
        for ch in script[..location].chars() {
            if ch == '\n' {
                current_column = 0;
            } else if ch == '\t' {
                current_column += 4;
            } else {
                current_column += 1;
            }
        }
        current_column
    };

    // Trim the beginning of the line and subtract the number of spaces from the column
    let trimmed_line = line.trim_start();
    column -= (line.len() - trimmed_line.len()) as i32;

    (line_number, String::from(trimmed_line), column as usize)
}

/// This is used to take an LALRPOP error and convert
/// it into a nicely formatted error message
fn format_error<T: core::fmt::Debug>(script: &str, err: Error<T>) -> String {
    match err {
        Error::InvalidToken { location } => {
            let (line_number, line, column) = get_line(script, location);
            make_error(
                &line,
                &(script.as_bytes()[location] as char).to_string(),
                line_number,
                column,
            )
        }
        Error::UnrecognizedEOF { location, .. } => {
            let (line_number, line, _) = get_line(script, location);
            make_error(&line, "EOF", line_number, line.len())
        }
        Error::UnrecognizedToken { token, .. } => {
            // The start and end of the unrecognized token
            let start = token.0;
            let end = token.2;

            let (line_number, line, column) = get_line(script, start);
            let unexpected = &script[start..end];
            make_error(&line, unexpected, line_number, column)
        }
        Error::ExtraToken { token } => {
            // The start and end of the extra token
            let start = token.0;
            let end = token.2;

            let (line_number, line, column) = get_line(script, start);
            let unexpected = &script[start..end];

            make_error(&line, unexpected, line_number, column)
        }
        Error::User { error } => format!(
            "  |\n? | {}\n  | {}\n  |\n  = unexpected compiling error",
            error,
            format!("^{}", "-".repeat(error.len() - 1))
        ),
    }
}
