
## Example Code

```rust
// code from "std/prelude.ok" ...
// code from "std/io.ok" ...

fn double(n: 1) -> 1 { n 2 * }

fn triple(n: 1) -> 1 { n 3 * }

fn main() -> 1 {
    5 double! x: 1 =
    6 triple! y: 1 =
    
    x prn!
    "*2 + " prs!
    y prn!
    "*3 = " prs!
    x y + prn!

    0
}
```
## Loops

```rust
fn main() -> 1 {
    for (0 i:int =; i 10 <; 1 i +=) {
        i prn!
    }
}
```

## Structures

```rust
// code from "std/prelude.ok" ...
// code from "std/io.ok" ...

fn Date::new() -> &1 { 3 new! }
fn Date::drop(self: &3) -> 0 {}

fn Date::month(self: &3) -> &1 { self }
fn Date::day(self: &3) -> &1   { self 1 + }
fn Date::year(self: &3) -> &1  { self 2 + }

fn Date::set(self: &3, month: 1, day: 1, year: 1) -> &3 {
    month self Date::month! =
    day   self Date::day!   =
    year  self Date::year!  =

    self
}

fn Date::tomorrow(self: &3) -> &3 {
    1 self Date::day! +=
    self
}

fn Date::yesterday(self: &3) -> &3 {
    1 self Date::day! -=
    self
}

fn Date::next_year(self: &3) -> &3 {
    1 self Date::year! +=
    self
}

fn Date::last_year(self: &3) -> &3 {
    1 self Date::year! -=
    self
}

fn print(self: &3) {
    self Date::month! @ prn!
    "/" prs!;
    self Date::day!   @ prn!
    "/" prs!;
    self Date::year!  @ prn!
}


fn main() -> 1 {
    2002 14 5 Date::new! Date::set! date: Date =

    date& Date::tomorrow! Date::print!
    9
}
```