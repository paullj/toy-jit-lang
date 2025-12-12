# If Statements

If statements, with optional `else if` and `else` branches:

```toy
if my_bool {
    my_int = add(2, 3)
} else if my_int > 10 and my_float < 5.0 {
    my_int = add(5, 7)
}
```

If statements can also be used as expressions:

```toy
my_int = if my_bool { 1 } else { 0 }
```
