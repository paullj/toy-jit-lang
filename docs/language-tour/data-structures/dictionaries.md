# Dictionaries

Dictionaries in `toy` are collections of key-value pairs, where each key is unique and maps to a specific value. They can be defined using curly braces `{}`, with key-value pairs separated by commas and a colon `:` separating keys from values:

```toy
my_dict: {string: int} = {
    "one": 1,
    "two": 2,
    "three": 3
}

Dictionary type annotations use the syntax `{KeyType: ValueType}`, where `KeyType` is the type of the keys and `ValueType` is the type of the values in the dictionary. In the example above, `my_dict` is a dictionary with string keys and integer values. The type annotation is optional if the type can be inferred from the context:

```toy
my_dict = {
    "one": 1,
    "two": 2,
    "three": 3
}  # inferred as {string: int}
```

Dictionaries in `toy` are mutable, allowing you to add, remove, or modify key-value pairs after creation. They live on the heap and are managed by the garbage collector.

You can create an empty dictionary, but because types can not be inferred this way, you must specify the key and value types:

```toy
empty_dict: {string: int} = {}
```

Elements in a dictionary can be accessed using their keys:

```toy
value := my_dict["two"]  # 2
```

You can add or update key-value pairs in a dictionary:

```toy
my_dict["four"] = 4        # Adds a new key-value pair
my_dict["two"] = 22        # Updates the value for the key "two"
```

<!-- TODO: methods on built in types -->
You can remove key-value pairs using the `remove` method:

```toy
my_dict.remove("three")    # Removes the key "three" and its value
```

You can check if a key exists in the dictionary using the `in` keyword:

```toy
if "one" in my_dict {
    echo "Key 'one' exists"
}
```
<!-- TODO: later -->
To get the number of key-value pairs in a dictionary, you can use the `len` function:

```toy
length := my_dict.len()  # 3
```

and iterate over the key-value pairs using a `for` loop:

```toy
for key, value in my_dict {
    echo value
}
```
