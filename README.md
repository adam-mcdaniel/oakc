# Oak

An infinitely more portable alternative to the C programming language.

![Example](assets/example.png)

## Why Oak?

For those of you that remember ["free"](https://github.com/adam-mcdaniel/free), oak is essentially a more robust and high level version of that project. The goal of oak is to be as high level as possible in the frontend, but as small and low level as possible in the backend.

#### About the Author

I'm a freshly minted highschool graduate and freshman in college looking for work. If you enjoy my projects, consider supporting me by buying me a coffee! 

<a href="https://www.buymeacoffee.com/adammcdaniel" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/default-violet.png" height=41px width=174px style="!important;box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;-webkit-box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;" ></a>

## Intermediate Representation

The key to oak's insane portability is its incredibly compact backend implementation. _The code for Oak's backend can be expressed in under 100 lines of C._ Such a small implementation is only possible because of the tiny instruction set of the intermediate representation. Oak's IR is only composed of **_14 different instructions_**. That's on par with [brainfuck](https://esolangs.org/wiki/Brainfuck)!

The backend of oak functions very simply. Every instruction operates on a _memory tape_. This tape is essentially a static array of double-precision floats.

```js
      let x: num = 5.25;    ...     let p: &num = &x;  `beginning of heap`
          |                             |                      |
          v                             v                      v
[0, 0, 0, 5.25, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, ...]
                                                       ^
                                                       |
                          `current location of the stack pointer`
```

When a variable is defined, it's given a static location on the memory tape. Then, the compiler just replaces the variable with its address in the rest of the code!

Additionally, the memory tape functions as a **_stack_** and a **_heap_**. After space for all of the program's variables is assigned, the memory used for the stack begins. The stack _grows_ and _shrinks_ with data throughout the program: when two numbers are summed, for example, they are popped off of the stack and replaced with the result. Similarly, the heap grows and shrinks throughout the program. The heap, however, is used for _dynamically allocated_ data: information with a memory footprint **unknown at compile time**.

Now that you understand how oak's backend fundamentally operates, here's the complete instruction set!

| Instruction | Side Effect |
|-|-|
| `push(n: f64);` | Push a number onto the stack. |
| `add();` | Pop two numbers off of the stack, and push their sum. |
| `subtract();` | Pop two numbers off of the stack. Subtract the first from the second, and push the result. |
| `multiply();` | Pop two numbers off of the stack, and push their product. |
| `divide();` | Pop two numbers off of the stack. Divide the second by the first, and push the result. |
| `sign();` | Pop a number off of the stack. If it is greater or equal to zero, push `1`, otherwise push `-1`. |
| `allocate();` | Pop a number off of the stack, and return a pointer to that number of free cells on the heap. |
| `free();` | Pop a number off of the stack, and go to where this number points in memory. Pop another number off of the stack, and free that many cells at this location in memory. |
| `store(size: i32);` | Pop a number off of the stack, and go to where this number points in memory. Then, pop `size` numbers off of the stack. Store these numbers in reverse order at this location in memory. |
| `load(size: i32);` | Pop a number off of the stack, and go to where this number points in memory. Then, push `size` number of consecutive memory cells onto the stack. |
| `call(fn: i32);` | Call a user defined function by it's compiler assigned ID. |
| `call_foreign_fn(name: String);` | Call a foreign function by its name in source. |
| `begin_while();` | Start a while loop. For each iteration, pop a number off of the stack. If the number is not zero, continue the loop. |
| `end_while();` | Mark the end of a while loop. |

Using only these instructions, oak is able to implement _**even higher level abstractions than C can offer**_!!! That might not sound like much, but it's very powerful for a language this small.

## Compilation Process

So how exactly does the oak compiler work?

1. Flatten structures into their functions
    - Structures in oak work differently than in other languages. The objects themselves are only arrays of memory cells: they don't have _**any**_ members or attributes. Structures _exclusively_ retrieve their data by using **_methods_** to return the addresses of their _"members"_. These methods are then flattened into simple functions. So, _`putnumln(*bday.day)`_ becomes _`putnumln(*Date::day(&bday))`_. This is a pretty simple process.

2. Calculate the size of every operation's type
    - Because of the structure of oak's intermediate representation, the type of every expression must be known for compilation to continue. The compiler combs over each expression and find's the size of its type. From here on, the representation of the code looks like this:

```rust
// `3` is the size of the structure on the stack
fn Date::new(month: 1, day: 1, year: 1) -> 3 {
    month; day; year
}
// self is a pointer to an item of size `3`
fn Date::day(self: &3) -> &1 { self + 1 }

fn main() -> 0 {
    let bday: 3 = Date::new(5, 14, 2002);
}
```

3. Statically compute the program's memory footprint
    - After totalling all the statically allocated data, such as the overall memory size of variables and string literals, the program preemptively sets aside the proper amount of memory on the stack. This essentially means that the stack pointer is _immediately_ moved to make room for all the data at the start of the program.

4. Convert Oak expressions and statements into equivalent IR instructions
    - Most expressions are pretty straightforward: function calls simply push their arguments onto the stack in reverse order and call a function by it's ID, references to a variable just push their assigned location on the stack as a number, and so on. Method calls, _however_, are a bit tricky.

    There are **_many_** different circumstances where a method call is valid. Methods _**always take a pointer to the structure as an argument**_. However, _an object that calls a method is not required to be a pointer_. For example, the following code is valid: _`let bday: Date = Date::new(); bday.print();`_. The variable `bday` is not a pointer, yet the method _`.print()`_ can still be used. Here's why.

    When the compiler sees a flattened method call, it needs to find a way to transform the "instance expression" into a pointer. For variables, this is easy: just add a reference! For instance expressions that are already pointers, it's even easier: don't do anything! For any other kind of expression, though, it's a bit more verbose. The compiler sneaks in a hidden variable to store the expression, and then compiles the method call again using the variable as the instance expression. Pretty cool, right?

5. Assemble the IR instructions for a target
    - Because oak's IR is so small, it can support several targets. Even better, adding a target is incredibly easy. In oak's crate, there's a trait named `Target`. If you implement each of the IR's instructions for your language using the `Target` trait, then oak can automatically compile all the way down to your new programming or assembly language! _Yes, it's as easy as it sounds!_

## Syntax and Flags

The syntax of oak is heavily inspired by the Rust programming language.

```rust
// An optional flag to set the exact number of memory cells to use for the heap.
// This makes Oak an extremely suitable language for embedded development!
#[heap(128)]

type bool(1) {
    fn true()  -> bool { return 1 as bool }
    fn false() -> bool { return 0 as bool }

    fn val(self: &bool) -> &num { return self as &num }

    fn not(self: &bool) -> bool {
        let result: bool = bool::true();
        // "self->val" is equivalent to "*self.val()"
        if self->val { result = bool::false(); }
        return result
    }
}

fn main() {
    putnumln(square(5));

    let b = bool::false();
    putboolln(b);
    // assign to b's "val" attribute
    b->val = 1;
    putboolln(b);
    b = bool::true();
    putboolln(b);

    let size: num = 32;
    // Allocate 32 cells
    let addr: &char = alloc(size);
    // Free those 32 cells
    free addr: size;
}


fn putbool(b: bool) {
    if b {
        putstr("true");
    } else {
        putstr("false");
    }
}

fn putboolln(b: bool) {
    putbool(b); putchar('\n');
}

// Functions can be ordered independently
fn square(x: num) -> num {
    putstr("Squaring the number '");
    putnum(x);
    putcharln('\'');
    // The last statement in a body doesn't require brackets
    return x * x
}
```

## Sample Output

Now it's time to show you the fruits of my labor!
Here's an example program.

```rust
fn fact(n: num) -> num {
    if n - 1 { n * fact(n-1) }
    else { 1 }
}

fn main() {
    prn!(fact(5))
}
```

Here is the same program compiled to C.
```c
void fn0(machine* vm);
void fn1(machine* vm);

void fn0(machine* vm) {
    machine_push(vm, 0);
    machine_store(vm, 1);
    machine_push(vm, 0);
    machine_load(vm, 1);
    machine_push(vm, 1);
    machine_subtract(vm);
    machine_push(vm, 1);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_push(vm, 2);
    machine_store(vm, 1);
    machine_push(vm, 1);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 0);
        machine_load(vm, 1);
        machine_push(vm, 0);
        machine_load(vm, 1);
        machine_push(vm, 1);
        machine_subtract(vm);
        fn0(vm);
        machine_multiply(vm);
        machine_push(vm, 0);
        machine_push(vm, 1);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_store(vm, 1);
        machine_push(vm, 1);
        machine_load(vm, 1);
    }
    machine_push(vm, 2);
    machine_load(vm, 1);
    while (machine_pop(vm)) {
        machine_push(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 1);
        machine_store(vm, 1);
        machine_push(vm, 0);
        machine_push(vm, 2);
        machine_store(vm, 1);
        machine_push(vm, 2);
        machine_load(vm, 1);
    }
}

void fn1(machine* vm) {
    machine_push(vm, 5);
    fn0(vm);
    prn(vm);
}

int main() {
    machine *vm = machine_new(3, 515);
    fn1(vm);

    machine_drop(vm);
    return 0;
}
```

That's quite a bit of output code for such a small program. How did our code get turned into this? First, our `fact` function was renamed as `fn0`.

Next, the function stores a number on the stack in the variable `n`:

```c
// `n` is stored in address 0
// store a number in address 0
machine_push(vm, 0);
machine_store(vm, 1);
```

Then, compute `n-1` by loading the value `n`, pushing the value `1`, and executing the subtract function.

```c
// `n` is stored in address 0
// load a number from address 0
machine_push(vm, 0);
machine_load(vm, 1);

machine_push(vm, 1);
machine_subtract(vm);
```

Store the result of `n-1` in a variable to use in the "if else" statement code, and `1` in another variable to determine if the "else" branch will run.

```c
// store `n-1` in address 1
machine_push(vm, 1);
machine_store(vm, 1);
// store `1` in address 2
machine_push(vm, 1);
machine_push(vm, 2);
machine_store(vm, 1);
```

Then, load the condition variable for the if statement and start the conditional branch.

```c
// load `n-1` off of the stack as the condition
machine_push(vm, 1);
machine_load(vm, 1);
// begin a while loop
while (machine_pop(vm)) {
    // load `n`
    machine_push(vm, 0);
    machine_load(vm, 1);

    // push `n-1` onto the stack
    machine_push(vm, 0);
    machine_load(vm, 1);
    machine_push(vm, 1);
    machine_subtract(vm);

    // call `fact` with `n-1`
    fn0(vm);

    // multiply the result of `fact(n-1)` with `n`
    machine_multiply(vm);

    // store zero in the while loop's condition variable
    // to stop the if statement's body from looping
    machine_push(vm, 0);
    machine_push(vm, 1);
    machine_store(vm, 1);

    // store zero in the "else" branch condition
    // so the else branch will not execute
    machine_push(vm, 0);
    machine_push(vm, 2);
    machine_store(vm, 1);

    // this loads the condition for the while loop, which
    // has been set to zero.
    machine_push(vm, 1);
    machine_load(vm, 1);
}
// end the if case
```

Then, check for the "else" case of the "if else" statement.
```c
// load the "else" case condition variable.
// if the "if" case executed, then this is zero
machine_push(vm, 2);
machine_load(vm, 1);
// begin else case conditional branch
while (machine_pop(vm)) {
    // push 1 onto the stack
    machine_push(vm, 1);

    // store zero in the if case condition variable
    machine_push(vm, 0);
    machine_push(vm, 1);
    machine_store(vm, 1);


    // store zero in the else case condition variable
    machine_push(vm, 0);
    machine_push(vm, 2);
    machine_store(vm, 1);

    // this loads the condition for the while loop, which
    // has been set to zero.
    machine_push(vm, 2);
    machine_load(vm, 1);
}
```

Lastly, in the entry point, our `fact` function is called with the argument `5`.

```c
void fn1(machine* vm) {
    machine_push(vm, 5);
    fn0(vm);
    // print the result of `fact(5)`
    prn(vm);
}
```

## Usage

The best way to install Oak is with the Rust package manager.

```bash
# Also works for updating oakc
cargo install -f oakc
```

Then, oak files can be compiled with the oakc binary.

```bash
oak -c examples/hello_world.ok
main.exe
```

### Dependencies

**C backend**
    - Any GCC compiler that supports C99

**Go backend**
    - Golang 1.14 compiler
