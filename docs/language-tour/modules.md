# Modules

Modules allow you to organize code into separate files and control what is visible to other parts of your program. Each file in `toy` is automatically a module, with the file name determining the module name.

## Visibility

By default, all declarations in a module are private. To make something accessible from other modules, use the `pub` keyword:

```toy
pub fn add(a: int, b: int): int {
    a + b
}

fn helper(x: int): int {
    x * 2
}
```

In this example, `add` and `PI` are public and can be used by other modules, while `helper` is private and can only be used within this file.

## Importing

To use code from another module, use the `use` statement. The prefix indicates where the module comes from:

```toy
use std.io          # standard library
use pkg.http        # installed package
use .helpers        # relative to current file
use src.helpers     # absolute from project root
```

You can import specific items from a module using curly braces:

```toy
use std.math.{sin, cos, PI}

result := sin(PI)
```

To import a module under a different name, use `as`:

```toy
use std.math as m

result := m.sin(m.PI)
```

## Re-exporting

Sometimes you want to expose items from another module as part of your own module's public API. Use `pub use` to re-export:

```toy
pub use std.math.PI
```

Now any module that imports yours will also have access to `PI`.

## File Structure

Since each file is a module, a typical project might look like:

```
src/
├── main.toy
├── math.toy
└── utils/
    ├── helpers.toy
    └── strings.toy
```

Where `main.toy` can use local modules:

```toy
use .math               # relative: ./math.toy
use src.utils.helpers   # absolute: src/utils/helpers.toy
```
