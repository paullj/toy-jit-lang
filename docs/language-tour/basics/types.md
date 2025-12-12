# Primitive Types

## Numbers

The language supports two primary numeric types: `int` for integers and `float` for floating-point numbers.

### Floats

```toy
float my_float := 3.14
float my_float := 2.5e10
float my_float := 1_000.50
float my_float := 0.000_123
```

### Integers

```toy
int my_int := 5
int my_int := 5_000

int my_int := 0b1010      # Binary
int my_int := 0o12        # Octal
int my_int := 0xA         # Hexadecimal
```



## Booleans

```toy
bool my_bool := false
```

## Strings

```toy
string my_string := "Hello, World!"
string my_string := "Line 1\nLine 2\tTabbed"
string my_string := "This is a \"quoted\" word."
string my_string := "This is a backslash: \\"

multiline_string := """This is a multi-line
string that spans multiple lines.
It preserves whitespace and line breaks."""

```
