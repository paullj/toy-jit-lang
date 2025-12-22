# `if` Statements

`if` statements in allow for conditional execution of code blocks. The syntax is similar to many other programming languages:

```toy
if condition {
    # code to execute if condition is true
}
```

An optional `else` block can be added to execute code when the condition is false:

```toy
if condition {
    # code to execute if condition is true
} else {
    # code to execute if condition is false
}
```

Multiple conditions can be checked using `else if`:

```toy
if condition1 {
    # code to execute if condition1 is true
} else if condition2 {
    # code to execute if condition2 is true
} else {
    # code to execute if neither condition1 nor condition2 is true
}
```

`if` statements can also be used as expressions that return values. The last expression in the executed block is returned:

```toy
result := if condition {
    value_if_true
} else {
    value_if_false
}
```

This allows for concise conditional assignments:

```toy
max_value := if a > b { a } else { b }
```

Since, the language is statically typed, the types of the values in the `if` and `else` blocks must be compatible.
