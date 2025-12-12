# Variables

Variables are defined with the `:=` operator.

```toy
int x := 5
```

Most of the time, types can be omitted and inferred.

```toy
y := true
```

Reassigning variables is done with the `=` operator.

```toy
x = 10
```

## Constants

Constants are defined with the `.=` operator. The convention is to use fully uppercase names for constants.

```toy
A .= 5
```

Constants cannot be reassigned and must be initialized at the time of declaration. Constants can be computed from other constants.

```toy
B .= A + 10
```