use oak::parse;
use std::fs::write;
fn main() {
    write(
        "output.c",
        parse(
//             r#"
// fn not(x: num) -> num {
//     let result: num = 1;
//     if x { result = 0; }
//     result;
// }

// fn neq(a: num, b: num) -> num {
//     not(eq(a, b));
// }

// fn eq(a: num, b: num) -> num {
//     let result: num = 1;
//     if a - b {
//         result = 0;
//     }
//     result;
// }

// fn factorial(n: num) -> num {
//     if eq(n, 1) { 1; }
//     if neq(n, 1) { n*factorial(n-1); }
// }

// fn strcpy(dst: &char, src: &char) -> void {
//     for (let i:num=0; src[i]; i=i+1;) {
//         dst[i] = src[i];
//     }
//     dst[i] = 0;
// }

// fn main() -> void {
//     prn!(factorial(5)); prend!();

//     let size: num = 10;
//     let addr: &num = alloc(size);

//     let s: &char = "test\n";
//     strcpy(addr, s);
//     prs!(addr);
//     prs!(s);
//     free addr : size;
    
//     prs!(s);
// }
// "#,
            r#"
fn strlen(str: &char) -> num {
    for (let i: num=0; str[i]; i=i+1;) {}
    i;
}

fn strcpy(dst: &char, src: &char) -> void {
    for (let i: num=0; src[i]; i=i+1;) {
        dst[i] = src[i];
    }
    dst[i] = 0;
}

fn strcat(dst: &char, src: &char) -> void {
    let offset: num = strlen(dst);
    for (let i: num=0; src[i]; i=i+1;) {
        dst[offset+i] = src[i];
    }
    dst[offset+i] = 0;
}

fn main() -> void {
    let size: num = 8;
    let s: &char = alloc(size);
    strcpy(s, "test");
    prs!(s); prend!();
    strcat(s, "ing");
    prs!(s); prend!();
    free s: size;
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
