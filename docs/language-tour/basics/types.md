# Primitive Types

## Ints

An `int` represents positive and negative whole numbers.

Underscores are allowed for better readability [^2], the following are all equivalent:

```toy
1_000_000
10000_00
1000000
```

Binary, octal, and hexadecimal integer literals are also supported:

```toy
0b1010      # Binary
0o12        # Octal
0xA         # Hexadecimal
```

[^2]: Underscores can be placed anywhere between digits in a number literal to improve readability. They do not affect the actual value of the number.

There are arithmetic operations available for `int` types that you would expect, such as addition, subtraction, multiplication, division, and modulus. 

```toy
10 + 5
10 - 5
10 * 5
10 / 5
10 % 5
```

> [!NOTE]
> Division between two integers results in an integer, with any fractional part truncated. For example, `7 / 3` yields `2`.

Comparison operators are also available for `int` types:

```toy
3 > 1 + 1
2 < 1 - 1
8 >= 1 + 3
8 <= 5 - 3
```

Equality operators can be used to compare `int` values:

```toy
5 == 2 + 3
4 != 2 * 3
```

## Floats

A `float` represents real numbers with decimal points.

It is defined by including a decimal point in a numeric literal. Similar to `int`, underscores are allowed for better readability:

```toy
1000.0
1_000.00
```

They can also be declared with scientific notation:

```toy
2.5e10
1.0e-3
```

Just like with `int`, arithmetic operations are available for `float` types. However, the operator is followed with a `.` to indicate that the operation is for floats:

```toy
10.0 +. 5.5
10.0 -. 5.5
10.0 *. 5.5
10.0 /. 5.5
```

Comparison operators are also available for `float` types:

```toy
3.5 >. 1.5 +. 1.0
2.0 <. 1.0 -. 5.0
8.0 >=. 1.0 +. 3.0
8.0 <=. 5.0 -. 3.0
```

Equality operators can be used to compare `float` values:

```toy
5.5 == 2.5 +. 3.0
4.0 != 2.0 *. 3.0
```

> [!WARNING]
> Due to the nature of floating-point arithmetic, direct equality comparisons between `float` values may not always yield expected results. It is often better to check if the difference between two `float` values is within a small epsilon range.

## Bools

A `bool` represents a value, which can be either `true` or `false`.`

```toy
true
false
```

Boolean values can be combined using logical operators:

```toy
true and false
true or false
!true
```

## Strings

A `string` is a sequence of characters enclosed in double quotes `"`.

```toy
"Hello, World!"
```

Several escape sequences are supported:

* \" - double quote
* \\ - backslash
* \f - form feed
* \n - newline
* \r - carriage return
* \t - tab
* \u{xxxxxx} - unicode codepoint

Multi-line strings can be created using triple double quotes `"""`:

```toy
"""This is a multi-line string.
It can span multiple lines.
"""
```

Multi-line strings preserve all whitespace and newlines within the triple quotes.