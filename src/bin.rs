use oak::parse;
use std::fs::write;
fn main() {
    write(
        "output.c",
        parse(
            r#"
fn not(x: num) -> num {
    let result: num = 1;
    if x { result = 0; }
    result;
}

fn neq(a: num, b: num) -> num {
    not(eq(a, b));
}

fn eq(a: num, b: num) -> num {
    let result: num = 1;
    if a - b {
        result = 0;
    }
    result;
}

fn factorial(n: num) -> num {
    if eq(n, 1) { 1; }
    if neq(n, 1) { n*factorial(n-1); }
}

fn main() -> void {
    prn!(factorial(5)); prend!();
}
"#,
        )
        .compile()
        .unwrap()
        .assemble()
        .unwrap()
        .assemble()
        .unwrap(),
    )
    .unwrap();
}
