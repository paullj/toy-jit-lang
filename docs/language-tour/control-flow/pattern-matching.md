# Pattern Matching

Match expressions are supported:

```toy
match my_var {
    true -> 3
    false -> {
        5
    }
    _ -> 12
}
```

Match expressions can also be used as expressions:

```toy
my_int = match my_bool {
    true -> 42
    false -> 24
}
```
