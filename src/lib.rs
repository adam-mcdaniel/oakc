#![allow(warnings, clippy, unknown_lints)]
use std::{
    collections::BTreeMap,
    env::consts::{FAMILY, OS},
    io::Result,
    path::PathBuf,
    process::exit,
};
pub type Identifier = String;
pub type StringLiteral = String;

pub mod asm;
pub mod hir;
pub mod mir;
use hir::{HirConstant, HirProgram};

mod target;
pub use target::{Go, Target, C};

use asciicolor::Colorize;
use comment::cpp::strip;
use time::OffsetDateTime;

use lalrpop_util::{lalrpop_mod, ParseError};
lalrpop_mod!(pub parser);

pub fn compile(cwd: &PathBuf, input: impl ToString, target: impl Target) -> Result<()> {
    let mut constants = BTreeMap::new();

    constants.insert(
        String::from("TARGET"),
        HirConstant::Float(target.get_name() as u8 as f64),
    );
    constants.insert(
        String::from("IS_STANDARD"),
        HirConstant::Float(target.is_standard() as i32 as f64),
    );

    constants.insert(
        String::from("ON_WINDOWS"),
        HirConstant::Float((OS == "windows") as i32 as f64),
    );
    constants.insert(
        String::from("ON_MACOS"),
        HirConstant::Float((OS == "macos") as i32 as f64),
    );
    constants.insert(
        String::from("ON_LINUX"),
        HirConstant::Float((OS == "linux") as i32 as f64),
    );

    constants.insert(
        String::from("ON_NIX"),
        HirConstant::Float((FAMILY == "unix") as i32 as f64),
    );
    constants.insert(
        String::from("ON_NON_NIX"),
        HirConstant::Float((FAMILY != "unix") as i32 as f64),
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

    match parse(input).compile(cwd, &target, &mut constants) {
        Ok(mir) => match mir.assemble() {
            Ok(asm) => match asm.assemble(&target) {
                // Add the target's prelude, the FFI code from the user,
                // the compiled Oak code, and the target's postlude
                Ok(result) => target.compile(target.prelude() + &result + &target.postlude()),
                Err(e) => {
                    eprintln!("compilation error: {}", e.bright_red().underline());
                    exit(1);
                }
            },
            Err(e) => {
                eprintln!("compilation error: {}", e.bright_red().underline());
                exit(1);
            }
        },
        Err(e) => {
            eprintln!("compilation error: {}", e.bright_red().underline());
            exit(1);
        }
    }
}

pub fn parse(input: impl ToString) -> HirProgram {
    let code = &strip(input.to_string()).unwrap();
    match parser::ProgramParser::new().parse(code) {
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
fn get_line(script: &str, location: usize) -> (usize, String, usize) {
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
