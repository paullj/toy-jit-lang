# Lists

Lists in `toy` are ordered collections of elements of the same type. They can be defined using square brackets `[]`, with elements separated by commas:

```toy
my_list: list[int] = [1, 2, 3, 4, 5]
```

List type annotations use the syntax `list[Type]`, where `Type` is the type of the elements in the list. In the example above, `my_list` is a list of integers. The type annotation is optional if the type can be inferred from the context:

```toy
my_list = [1, 2, 3, 4, 5]  # inferred as list[int]
```

Lists in `toy` are mutable, meaning you can change their contents after creation, but the type of the elements must remain consistent. They live on the heap and are managed by the garbage collector, allowing for dynamic resizing.

You can create an empty list by specifying the type:

```toy
empty_list: list[int] = []
```

Elements can be accessed using their index, with the first element at index 0:

```toy
first := my_list[0]  # 1
second := my_list[1]  # 2
last := my_list[-1]  # 5

empty := empty_list[0]  # Error: Index out of bounds
```

You can access a range of elements using the slice syntax:

```toy
sub_list := my_list[1..4]  # [2, 3, 4]
index_to_end := my_list[2..]  # [3, 4, 5]
beginning_to_index := my_list[..3]  # [1, 2, 3]

from_second_last := my_list[-3..]  # [3, 4, 5]
up_to_last := my_list[..-1]  # [1, 2, 3, 4]
```

To get the length of a list, you can use the `len` function:

```toy
length := my_list.len()  # 5
```

Lists can be modified by adding or removing elements:

```toy
my_list := [1, 2, 3]
my_list := my_list.push(4)       # [1, 2, 3, 4]
my_list := my_list.pop()         # [1, 2, 3], returns 4
my_list := my_list.remove(1)     # [1, 3, 4]
my_list := my_list.insert(1, 2)  # [1, 2, 3, 4]
my_list := my_list.clear()       # []
my_list.extend([5, 6, 7])        # [5, 6, 7]
my_list := my_list[1..4]         # [2, 3, 4]
```

# Future work
<!-- TODO:Implement this later -->

Lists can be iterated over using a `for` loop:

```toy
my_list := [1, 2, 3, 4, 5]
for item in my_list {
    print(item)
}
```

You can also use higher-order functions like `map`, `filter`, and `reduce` on lists:

```toy
my_list := [1, 2, 3, 4, 5]
squared := my_list.map(fn(x) { x * x })
evens := my_list.filter(fn(x) { x % 2 == 0 })
sum := my_list.reduce(0, fn(acc, x) { acc + x })
