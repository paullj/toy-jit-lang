# Function Definitions

Function definitions must specify parameter and return types:

```toy
fn add(int a, int b) int {
    int x := a
    y := 0
    y = b

    # The last expression is the return value
    x + y
}
```

Destructuring is also supported in function parameters:

```toy
fn sum_point(Point { x, y }: Point) {
    x + y
}
```
