# Vryn Language — Syntax Quick Spec

## Hello World
```vryn
fn main() {
    println("Hello, World!")
}
```

## Variables
```vryn
let name = "Sanjeev"          // immutable (default)
let mut age = 25               // mutable
let pi: f64 = 3.14159          // explicit type
```

## Functions
```vryn
fn add(a: i32, b: i32) -> i32 {
    a + b                       // last expression is return value
}

fn greet(name: str) {
    println("Hello, {name}!")   // string interpolation
}
```

## Control Flow
```vryn
// If-else (is an expression — returns value)
let status = if age >= 18 { "adult" } else { "minor" }

// Match (pattern matching)
match color {
    "red" => println("Stop!"),
    "green" => println("Go!"),
    "yellow" => println("Slow down!"),
    _ => println("Unknown color"),
}

// For loop
for i in 0..10 {
    println("{i}")
}

for item in list {
    println("{item}")
}

// While loop
while count > 0 {
    count -= 1
}
```

## Structs
```vryn
struct User {
    name: str,
    age: i32,
    email: str,
}

impl User {
    fn new(name: str, age: i32, email: str) -> User {
        User { name, age, email }
    }

    fn greet(self) {
        println("Hi, I'm {self.name}!")
    }
}
```

## Enums
```vryn
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

fn area(shape: Shape) -> f64 {
    match shape {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Triangle(a, b, c) => {
            let s = (a + b + c) / 2.0
            (s * (s-a) * (s-b) * (s-c)).sqrt()
        }
    }
}
```

## Option & Result (No Null!)
```vryn
fn find_user(id: i32) -> Option<User> {
    if id == 1 {
        Some(User::new("Sanjeev", 25, "sanjeev@mail.com"))
    } else {
        None
    }
}

fn read_file(path: str) -> Result<str, Error> {
    // ...
}

// Using ? operator for error propagation
fn process() -> Result<str, Error> {
    let content = read_file("data.txt")?
    let parsed = parse(content)?
    Ok(parsed)
}
```

## Pipe Operator
```vryn
let result = data
    |> filter(|x| x > 0)
    |> map(|x| x * 2)
    |> sum()

// Equivalent to: sum(map(filter(data, |x| x > 0), |x| x * 2))
```

## Traits
```vryn
trait Display {
    fn to_string(self) -> str
}

impl Display for User {
    fn to_string(self) -> str {
        "{self.name} (age: {self.age})"
    }
}
```

## Concurrency
```vryn
spawn {
    println("Running in parallel!")
}

// Channels
let (tx, rx) = channel<i32>()
spawn {
    tx.send(42)
}
let value = rx.recv()
```

## Keywords
fn, let, mut, if, else, match, for, while, loop, break, continue,
return, struct, enum, trait, impl, pub, use, spawn, async, await,
true, false, self, in, as, type, const, static, mod, where
```
