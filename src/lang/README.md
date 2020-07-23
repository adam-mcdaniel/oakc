


## Example Code

```rust
include!("std/prelude.ok")
include!("std/io.ok")

fn double(n: float) -> float {
    n * 2
}

fn triple(n: float) -> float {
    n * 3
}

fn main() -> int {
    let x: float = double(5);
    let y: float = triple(6);

    prn(x);
    prs("*2 + ");
    prn(y);
    prs("*3 = ");
    prn(x+y);

    0
}
```


## Structures

```rust
@include("std/prelude.ok")
@include("std/io.ok")


@define("DATE", add(INT, INT, INT))
struct Date(DATE) {
    fn new() -> &Date { new(DATE) }
    fn drop(&self) {}

    fn set(&self, month: int, day: int, year: int) -> &Date {
        self->month = month;
        self->day = day;
        self->year = year;
        self
    }

    fn month(&self) -> &int { self }
    fn day(&self) -> &int { self + 1 }
    fn year(&self) -> &int { self + 2 }

    fn tomorrow(&self) -> &Date {
        self->day += 1; self
    }

    fn yesterday(&self) -> &Date {
        self->day -= 1; self
    }

    fn next_year(&self) -> &Date {
        self->year += 1; self
    }

    fn last_year(&self) -> &Date {
        self->year -= 1; self
    }

    fn print(&self) {
        prn(self->month); prs("/");
        prn(self->day); prs("/");
        prn(self->year);
    }
}


fn main() -> int {
    let date: Date = *Date::new().set(5, 14, 2002);

    date.tomorrow().print();
}
```