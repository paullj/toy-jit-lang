# Structs

Structs in `toy` are user-defined types that group related data together. They are defined using the `struct` keyword, followed by the struct name and its fields:

```toy
struct Point {
    x: int
    y: int
}
```

You can create instances of a struct by calling its constructor with the required field values:

```toy
my_point: Point = Point { x: 3, y: 4 }
```

Struct fields can be accessed using dot notation:

```toy
echo my_point.x  # 3
echo my_point.y  # 4

Structs are mutable by default, allowing you to modify their fields after creation:

```toy
my_point.x = 10
echo my_point.x  # 10
```

You can also create a new struct instance by copying an existing one and modifying specific fields:

```toy
new_point = Point { ..my_point, y: 20 }
echo new_point.x  # 10
echo new_point.y  # 20
```

When the variable names match the field names, you can use shorthand syntax for construction:

```toy
x = 5
y = 15
p = Point { x, y }  # same as Point { x: x, y: y }
```
