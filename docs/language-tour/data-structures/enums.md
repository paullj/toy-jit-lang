# Enums

Enums in `toy` are user-defined types that represent a fixed set of named values, known as variants. They are defined using the `enum` keyword, followed by the enum name and its variants:

```toy
enum Color {
    Red,
    Green,
    Blue
}
```

Enum variants can also have associated data:

```toy
enum Shape {
    Circle(radius: float)
    Rectangle(width: float, height: float)
    Triangle(a: float, b: float, c: float)
}
```

You can create instances of an enum by specifying the variant and providing any required associated data:

```toy
my_color: Color = Color.Red
my_shape: Shape = Shape.Circle(5.0)
```

Enums can have methods defined within them. Methods are functions that operate on instances of the enum and have access to the enum's variants via the `self` keyword.

```toy
enum Color {
    Red
    Green
    Blue

    fn to_string(self): string {
        match self {
            Color.Red {
                return "Red"
            }
            Color.Green {
                return "Green"
            }
            Color.Blue {
                return "Blue"
            }
        }
    }
}

Enum variants can be accessed using pattern matching with the `match` expression:

```toy
match my_shape {
    Shape.Circle(radius) {
        echo "Circle with radius: " + radius.to_string()
    }
    Shape.Rectangle(width, height) {
        echo "Rectangle with width: " + width.to_string() + " and height: " + height.to_string()
    }
    Shape.Triangle(a, b, c) {
        echo "Triangle with sides: " + a.to_string() + ", " + b.to_string() + ", " + c.to_string()
    }
}
```
