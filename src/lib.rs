#![allow(warnings, clippy, unknown_lints)]

pub type Identifier = String;
pub type StringLiteral = String;

pub mod asm;
pub mod hir;
pub mod mir;
use hir::HirProgram;

use comment::python::strip;

use lalrpop_util::{lalrpop_mod, ParseError};
lalrpop_mod!(pub parser);

pub fn parse(input: &str) -> HirProgram {
    match parser::ProgramParser::new().parse(&strip(input).unwrap()) {
        // if the parser succeeds, build will succeed
        Ok(mut parsed) => {
            // parsed.set_heap_size(65536);
            println!("{:#?}", parsed);
            parsed
        }
        // if the parser succeeds, annotate code with comments
        Err(e) => {
            eprintln!("{}", format_error(&input.to_string(), e));
            panic!("compilation error");
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
        line = line,
        underline = underline,
        unexpected = unexpected
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
            let lines = script.lines().collect::<Vec<&str>>();
            lines[lines.len() - 1]
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
