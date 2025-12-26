# Implicit Line Termination

This document describes the rules for implicit statement termination.

## Core Rule

**Newline terminates a statement unless inside unclosed delimiters.**

## Rules

| Rule | Valid | Invalid |
|------|-------|---------|
| `{` must be on same line as construct | `if true {` | `if true\n{` |
| Incomplete expression errors at newline | - | `x := 1 +\n2` |
| Unclosed delimiters continue across newlines | `foo(1,\n2)` | - |
| Multiple newlines treated as one | `x := 1\n\n\ny := 2` | - |
| Comments end statements | `1 # comment\n+ 2` = two stmts | - |

## Unclosed Delimiters

These delimiters allow continuation across newlines when unclosed:

- `(` ... `)`
- `{` ... `}`
- `[` ... `]`

```toy
# Valid - unclosed paren continues
x := (1 +
      2)

# Valid - unclosed paren continues
x := (
    1 + 2
)

# Valid - function call with unclosed paren
foo(1,
    2,
    3)

# Valid - function call with unclosed paren
foo(1,
    2,
    3
)


# Valid - unclosed brace in block expression
x := {
    1 + 2
}
```

## Trailing Operators Do NOT Continue

Unlike some languages, a trailing operator does NOT continue to the next line.

```toy
# INVALID - error after +
x := 1 +
     2

# Must wrap in parens to continue
x := (1 +
      2)
```

The error is reported at the `2`.

## Block Constructs

All block constructs (`if`, `fn`, `for`, `while`, `loop`, `match`) require `{` on the same line:

```toy
# Valid
if condition {
    body
}

fn foo() int {
    1
}

# Invalid - brace on next line
if condition
{
    body
}
```

## Comments

Comments do NOT continue statements. A comment at end of line terminates the statement.

```toy
# This is TWO statements:
x := 1 # comment
+ 2    # this is a prefix expression, not continuation

# To continue, use parens:
x := (1 # comment ok inside parens
      + 2)
```

## Multiple Newlines

Multiple consecutive newlines are captured as trivia but treated as a single statement terminator:

```toy
# Valid - same as single newline between
x := 1


y := 2
```

## Multi-line Strings

TODO: Multi-line string literals (`"""..."""`) are not yet implemented.

When implemented, content inside triple quotes should preserve newlines as string content, not statement terminators.
