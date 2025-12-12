# Structs

Structs can be defined with the `type` keyword:

```toy
type Point = {
    float x
    float y
}
```

Instances of structs are created with the struct name followed by curly braces:

```toy
Point p := Point {
    x: 1.0
    y: 2.0
}
```

Struct fields are accessed with the `.` operator:

```toy
float px := p.x
float py := p.y
```

## Destructuring

Destructuring can also be used with structs:

```toy
Point { x: px, y: py } := p
```

Now `px` and `py` are assigned the values of `p.x` and `p.y` respectively:

```toy
float new_px := px
```

Destructuring is also supported in function parameters:

```toy
fn sum_point(Point { x, y }: Point) {
    x + y
}
```
