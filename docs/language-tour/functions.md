# Functions

Functions are defined using the `fn` keyword, followed by the function name, a list of parameters in parentheses, an optional return type, and a block of code enclosed in curly braces. The return keyword is used to specify the return value of the function.

```toy
fn add(a: int, b: int): int {
    return a + b
}
```

Like variable definition, type inference can be used to omit the return type and parameter types if they can be inferred from the context:

```toy
fn add(a, b) {
    return a + b
}

add(5, 10)
```

Similar to other expressions in `toy`, the last expression in a function is implicitly returned, so the `return` keyword in this case is optional:

```toy
fn add(a, b) {
    a + b
}
```

Functions can also have no return value, in which case they return the `unit` type, represented by `_`:

```toy
fn do_nothing(): _ {
    # This function does nothing
}

Functions can be called by using their name followed by parentheses containing any required arguments:

```toy
result := add(5, 10)
```

Anonymous functions can be defined using the `fn` keyword without a name. They can be assigned to variables or passed as arguments to other functions:

```toy
increment := fn(x) { x + 1 }

result := increment(5)
```

Higher-order functions can take other functions as parameters or return functions as results:

```toy
fn apply_twice(f, x) {
    f(f(x))
}

result := apply_twice(increment, 5)  # result is 7
```

Functions can also have default parameter values:

```toy
fn greet(name: string = "World") {
    return "Hello, " + name + "!"
}

greet()          # returns "Hello, World!"
greet("Alice")   # returns "Hello, Alice!"
```

Functions can be recursive, allowing them to call themselves.

```toy
fn factorial(n: int): int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
result := factorial(5)  # result is 120
```

Functions in `toy` support closures, meaning they can capture variables from their surrounding scope:

```toy
fn make_adder(x) {
    return fn(y) {
        x + y
    }
}

add_five := make_adder(5)
result := add_five(10)  # result is 15
```
