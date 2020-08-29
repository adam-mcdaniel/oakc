# tour

A tour of the Oak programming language.

# Table of Contents
1. [Syntax](#syntax)
2. [Constants](#constants)
2. [Expressions](#expressions)
2. [Statements](#statements)
3. [Compiler Directives](#compiler-directives)
4. [Functions](#functions)
5. [Structures](#structures)

## Syntax

The syntax of Oak is supposed to somewhat resemble Rust's. It's simple and easy on the eyes.

![Example](../assets/example.png)

Oak's syntax structures fall into one of three different categories:

|Declaration|Statement|Expression|
|-|-|-|
|Declarations are any kind of syntactic structure that can be written in global scope. These include [compiler directives](#compiler-directives), [structures](#structures), [functions](#functions), and [constants](#constants)|Statements are any kind of syntactic structure that can be written in a function body.|Expressions are any kind of syntactic structure that has an intrinsic value, like `1`, `variable`, or `get_char()`|

## Constants

Constants are simply named _constant expressions_. During compile time, constant names are replaced with their defined constant expressions.

Here's a table of all the different types of constant expressions.

|Expression|Type|Value|
|-|-|-|
|`true`|`bool`||
|`false`|`bool`||
|`!expr`|`bool`|Inverts a constant boolean expression.|
|`-expr`|`num`|Negates a constant number.|
|`lhs + rhs`|`num`|Adds two constants.|
|`lhs - rhs`|`num`|Subtracts two constants.|
|`lhs * rhs`|`num`|Multiplies two constants.|
|`lhs / rhs`|`num`|Divides two constants.|
|`lhs && rhs`|`bool`|Boolean AND two constants.|
|`lhs || rhs`|`bool`|Boolean OR two constants.|
|`lhs == rhs`|`bool`|Compare two constants.|
|`lhs != rhs`|`bool`||
|`lhs < rhs`|`bool`||
|`lhs <= rhs`|`bool`||
|`lhs > rhs`|`bool`||
|`lhs >= rhs`|`bool`||

Constant expressions can be used for any compiler directive that takes a constant argument. In addition to the different operators on constants, there are _**predefined constants**_.

|Expression|Type|Value|
|-|-|-|
|[`condition? then : else`](../examples/predef/predef_const.ok)|The type of `then`|If the constant `condition` expression is true, then replace this constant with `then`. Otherwise, replace this constant with `else`. |
|[`current_line()`](../examples/predef/predef_const.ok)|`num`|The line number in the file that this expression is evoked in. This is mainly intended for error reporting.|
|[`is_movable(type)`](../examples/predef/predef_const.ok)|`bool`|Whether or not a type uses its copy constructor or destructor. If a type _does_ use these methods, then `is_movable(type)` is false.|
|[`sizeof(type)`](../examples/predef/predef_const.ok)|`num`|Get the size of a type in memory.|
|[`is_defined("constant_name")`](../examples/predef/predef_const.ok)|`bool`|Has a given constant been defined?|
|[`ON_WINDOWS`](../examples/predef/predef_const.ok)|`bool`|Was this program compiled on Windows?|
|[`ON_MACOS`](../examples/predef/predef_const.ok)|`bool`|Was this program compiled on MacOS?|
|[`ON_LINUX`](../examples/predef/predef_const.ok)|`bool`|Was this program compiled on Linux?|
|[`ON_NIX`](../examples/predef/predef_const.ok)|`bool`|Was this program compiled on a \*nix system?|
|[`ON_NON_NIX`](../examples/predef/predef_const.ok)|`bool`|Was this program compiled on a non \*nix system?|
|[`DATE_DAY`](../examples/predef/predef_const.ok)|`num`|The day of the month this program was compiled on.|
|[`DATE_MONTH`](../examples/predef/predef_const.ok)|`num`|The month this program was compiled on.|
|[`DATE_YEAR`](../examples/predef/predef_const.ok)|`num`|The year this program was compiled on.|
|[`TARGET`](../examples/predef/predef_const.ok)|`char`|The single character identifier for the target backend this program is being compiled to. This can be either:|
||| - `'c'` for the C backend|
||| - `'g'` for the Go backend|
||| - `'t'` for the TypeScript backend|
|[`IS_STANDARD`](../examples/predef/predef_const.ok)|`bool`|Does the backend implement all the features of a standard Oak implementations? Backends for embedded devices might not implement some core features, such as floating point numbers, or other core and standard library functions.|

## Expressions



## Compiler Directives

Compiler directives are simple commands that instruct Oak to perform extra operations on the program before it's compiled. Most of them are called using the `#[directive(arg1, arg2)]` syntax.

Here's a comprehensive list of the different directives and what they do.

|Directive|Purpose|Example usage|
|-|-|-|
|[`header`](../examples/flags/doc.ok)|Adds a docstring to the top of the file. Multiple of these can be used, and they will contiously append to the docstring.|`#[header("This file implements the File IO")]`|
|[`doc`](../examples/flags/doc.ok)|Adds a docstring to a declaration.|`#[doc("PI constant for math")] const PI = 3.14159;`|
|[`std`](../examples/flags/require_std.ok)|Includes the standard library in the program. Without it, standard library functions cannot be used.|`#[std]`|
|[`no_std`](../examples/flags/no_std.ok)|Makes including the standard library _illegal_. You might use this if you redefine standard library functions, or if you're writing a library for a non-standard target.|`#[no_std]`|
|[`assert`](../examples/flags/assert.ok)|Assert that the value of a constant expression is true, or throw a compile time error.|`#[assert(2 + 2 == 4)]`|
|[`extern`](../examples/ffi/lib/foreign.ok)|Include a file containing foreign functions for a target. If `fs.c` implements a foreign function, you need to use the `extern` compiler directive to include the file and get access to the function.|`#[extern("fs.c")]`|
|[`include`](../examples/include/main.ok)|Include another Oak file's source code into this file.|`#[include("test.ok")]`|
|[`import`](../examples/import/main.ok)|Import another Oak file's definitions into this file. Behind the scenes, this just expands to an `include` directive with an `is_defined` guard.|`#[import("test.ok")]`|
|[`memory`](../examples/flags/mem_too_small.ok)|Set the number of words to use for _dynamic_ memory. Memory defined at _compile time_ is not affected **(such as memory allocated for string literals)**|`#[memory(512)]`|
|[`error`](../examples/flags/err.ok)|Throw a compile time error with a message for the user.|`#[error("This is a compile time user error")]`|
|`define`|Define a constant.|`#[define("BDAY_MONTH", 5)]`|
|[`if`](../examples/flags/if.ok)|Allows conditional compilation. This is used for compiling different code based on compile time conditions.|`#[if(TARGET == 'c') { const ABC = 0; }]`|
|[`if else`](../examples/flags/if.ok)||`#[if(TARGET == 'c') { const ABC = 0; } else { const ABC = 1; }]`|

## Functions

Functions are the smallest unit of abstraction in Oak. They are declared with the `fn` keyword as seen below.

```rust
// `test` takes the parameters `n` and `str`, which are of type `num` and `&char` respectively.
// `test` returns the type `bool`
fn test(n: num, str: &char) -> bool {
    // code here ...
}
```

Functions can return values with the `return` keyword.

```rust
#[std]

// Print a prompt, and get the users yes or no reply
fn yes_or_no(prompt: &char) -> bool {
    putstr(prompt);
    
    for (
        let ch = get_char();
        ch == '\n' || ch == '\r';
        ch = get_char();
    ) {}

    return ch == 'y' || ch == 'Y'
}
```

## Structures

Structures are _(currently)_ the only way to declare user defined types. They must have _at least_ one member, and can have associated functions (similar to static methods) and methods.

Methods always take a `self` argument, which is a pointer to the structure's type. `self` just points to the instance's data in memory. 

```rust
struct Date {
    // a Date object has the members `m`, `d`, and `y`,
    // which are all of type `num`
    let m: num,
        d: num,
        y: num;

    // This is called with `Date::new(...)`
    fn new(
        month: num,
        day: num,
        year: num
    ) -> Date {
        // Return a Date structure with
        return [month, day, year]
    }

    // This is called with `Date::birthday()`
    fn birthday() -> Date {
        return Date::new(5, 14, 2002)
    }

    fn tmrw(self: &Date) -> Date {
        let result = *self;
        // Add one to the `d` member of self
        // IMPORTANT NOTE:
        //     The arrow operator DOES NOT
        //     dereference the `result` variable.
        //     The arrow operator accesses members.
        result->d += 1;
        return result;
    }
}

fn yesterday(d: Date) -> Date {
    // Semicolons have to be used on every line except for the last line of the body
    d->d -= 1;
    return d
}
```

Structures can also implement copy constructors and destructors with the `copy` and `drop` methods. These allow structures to "automatically" manage their memory.

```rust
struct String {
    let contents: &char,
        len: num;
    
    ... Important functions here ...

    /// Copy constructors always take a pointer
    /// to the structure's type, and return a
    /// structure by value.
    fn copy(self: &String) -> String {
        let contents = alloc(self->len * sizeof(char)) as &char;

        // copy over the contents of this string
        memcpy(
            contents,
            self->contents,
            self->len + 1
        );

        // Add the null terminator
        contents[self->len] = '\0';
        return [contents, self->len]
    }

    /// Copy constructors always take a pointer
    /// to the structure's type, and return void
    fn drop(self: &String) {
        // free the contents of the string
        free self->contents: self->len + 1;
    }
}
```