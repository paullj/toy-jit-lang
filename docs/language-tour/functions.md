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
