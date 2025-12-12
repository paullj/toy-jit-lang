# Type Aliases

Type aliases can be created with the `type` keyword:

```toy
type NewInt = int
```

Aliases can not be used interchangeably with the original type:

```toy
NewInt my_alias_int := 42

# my_int = my_alias_int  # This would be a type error
```

## Generics

Generics are supported in type definitions and function definitions:

```toy
type Wrapper[T] = {
    T value
}

fn wrap[T](T val) Wrapper[T] {
    Wrapper[T] {
        value: val
    }
}
```
