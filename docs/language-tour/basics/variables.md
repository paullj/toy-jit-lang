# Variables an

## Variables

A variable is a container that our program uses to store a value in memory. They are declared with a name, a type, and a value. The `=` operator is used for assignment of variables and constants. The `:` operator is used for specifying the type of a variable when its declared.


To declare a variable `x` of type `int` and assign it the value `5`, you would write:

```toy
x: int = 5
```

A shorthand syntax for declaring and initializing variables is `:=`, where we can omit the type annotation [^1], and infer the type automatically.

```toy
x := 5
```

[^1]: Since `:` and `=` are actually separate operators, you can omit or include spaces between them, all of the following are equivalent:
    ```toy
    x: int = 5
    x :int= 5
    x:     = 5
    x := 5
    ```


> [!IMPORTANT]
> In practice, most code will use the `:=` operator for variable declaration since its easier to read and leverages type inference.

You can also declare multiple variables in a single line:

```toy
x, y  := 10, 20
```

Assigning to an existing variable is done with just the `=` operator:

```toy
x = 10
```

<!-- TODO: Make this syntax nicer to read, maybe not = but something else? > ~ | ! & -->
<!-- 
# Constants

Constants are values that cannot be changed once assigned. They are declared using a _similar_ syntax to variables, but use the `=!` operator instead of the `=` assignment operator.

```toy
x: int =! 10
```

Like variables, constants can also be declared using the shorthand syntax with type inference:

```toy
x :=! 10
```

Unlike variables, attempting to reassign a constant will result in a compile-time error:

```toy
x :=! 10
x = 20  # This is not allowed!
``` 
-->