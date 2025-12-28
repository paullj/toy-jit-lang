# Tuples

Tuples in `toy` are fixed-size collections of elements that can be of different types. They are defined using parentheses `()`, with elements separated by commas:

```toy
my_tuple: (int, string, bool) = (42, "hello", true)
```

Tuple type annotations use the syntax `(Type1, Type2, ...)`, where each `Type` is the type of the corresponding element in the tuple. In the example above, `my_tuple` is a tuple of an integer, a string, and a boolean. The type annotation is optional if the type can be inferred from the context:

```toy
my_tuple = (1, 2, 3)  # inferred as (int, int, int)
```

```toy
my_tuple = (1, 2, 3)
my_tuple[0] = 4  # Error: Cannot modify immutable tuple
```

Elements in a tuple can be accessed using their index, with the first element at index 0:

```toy
echo my_large_tuple.0  # 42
```

You can destructure a tuple into individual variables:

```toy
my_tuple := (1, "hello", true)
(a, b, c) := my_tuple
echo a  # 1
echo b  # "hello"
echo c  # true
```

Using `_` and slice notation to ignore certain elements:

```toy
my_tuple := (1, "hello", true)
(a, _, c) := my_tuple
echo a  # 1
echo c  # true

(x, ..) := my_tuple
echo x  # 1
```

Tuples can also be iterated over using a `for` loop:

```toy
my_tuple := (1, 2, 3, 4, 5)
for item in my_tuple {
    print(item)
}
```