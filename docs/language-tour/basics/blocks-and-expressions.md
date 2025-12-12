# Blocks and Expressions

Blocks are defined with curly braces.

Variables defined inside a block are scoped to that block.

```toy
{
    int temp := my_int * 2
    my_float = my_float + float(temp)
}

# temp is not accessible here
```

Blocks can also be used as expressions, returning the value of the last expression inside the block.

```toy
my_float := {
    my_float * 1.5
    my_float
}
```
