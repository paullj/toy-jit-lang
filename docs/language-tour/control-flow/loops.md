# `loop` statement

The `loop` statement creates an infinite loop that will continue executing until it is explicitly broken out of. The syntax is straightforward:

```toy
loop {
    # code to execute repeatedly
}
```

To exit a `loop`, you can use the `break` statement:

```toy
loop {
    # code to execute repeatedly
    if some_condition {
        break
    }
}
```

You can also use the `continue` statement to skip the current iteration and move to the next one:

```toy
loop {
    # code to execute repeatedly
    if some_other_condition {
        continue
    }
}
```

The `return` statement can be used within a `loop` to exit from the enclosing function and return a value:

```toy
loop {
    # code to execute repeatedly
    if some_condition {
        return value
    }
}
```

Loops can have identifiers to allow for nested loops and more control over breaking out of specific loops:

```toy
loop: outer_loop {
    loop: inner_loop {
        if some_condition {
            break outer_loop
        }
        if some_other_condition {
            break inner_loop
        }
    }
}
```


# `while`

`while` loops allow for repeated execution of a block of code while a condition is true. The syntax is similar to many other programming languages:

```toy
while condition {
    # code to execute while condition is true
}
```

Just like with `loop`, you can use the `continue`, `break` and `return` statements to skip the current iteration and move to the next one or exit the `while` loop entirely:

```toy

```toy
while condition {
    # code to execute while condition is true
    if some_condition {
        continue
    }
    if some_other_condition {
        break
    }
    if another_condition {
        return value
    }
}
```

`while` loops can also have identifiers for better control in nested scenarios:

```toy
while condition1: outer_while {
    while condition2: inner_while {
        if some_condition {
            break outer_while
        }
        if some_other_condition {
            break inner_while
        }
    }
}
```

# `for`

`for` loops are used to iterate over elements in a list or dictionary. The syntax is as follows:

```toy
for item in collection {
    # code to execute for each item in the collection
}
```

To iterate over a range of numbers, you can use the slice syntax which returns numbers from `start` to `end - 1`:

```toy
for item in start..end {
    # code to execute for each item in the range
}
```

You can also use `continue`, `break`, and `return` statements within a `for` loop:

```toy
for item in collection {
    if some_condition {
        continue
    }
    if some_other_condition {
        break
    }
    if another_condition {
        return value
    }
}
```

`for` loops can also have identifiers for better control in nested scenarios:

```toy
for item in collection: outer_for {
    for sub_item in item.sub_collection: inner_for {
        if some_condition {
            break outer_for
        }
        if some_other_condition {
            break inner_for
        }
    }
}
```
