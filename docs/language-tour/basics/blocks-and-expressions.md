# Blocks and Expressions

A block is a sequence of expressions surrounded by curly braces `{}`. Blocks define a scope for variables and control flow. The value of a block is the value of its last expression.

Variables defined inside a block are scoped to that block

```toy
x := 1
{
    y := x + 2
}
# The following line would cause an error because y is not defined outside the block
# z := y + 3
```

The value of a block is the value of its last expression

```toy
result := {
    a := 5
    b := 10
    a + b  # The value of the block is 15
}
```
