**toy**

toy is a simple programming language that is made for fun.

**goals**

* easy to learn
* helpful errors
* fast
* great tooling

**architecture**

* frontend
  * lexer
  * parser
  * ast
  * hir
  * mir
* middle
  * optimisations
* backend
  * jit compiler
  * register based vm
* tooling
  * cli
  * lsp server
  * vscode extension

**todo**
* bugs
  * defining a variable should return unit type, not the value assigned
    * not sure if this is a bug yet since we don't have functions yet so ignore
  * hover for variables doesn't show type in blocks
  * infer has some stuff in there which is not related to type inference

* more helpful warnings
  * doc comments which are not attached to anything
    * suggest attaching them to the next function/variable definition/doccomment
  * unused variables (naming convention: _varname to ignore warning)
  * unused functions (naming convention: _funcname to ignore warning)
  * shadowing variables
  * [not yet because all code is reachable] unreachable code

* garbage collection
  * reference counting?
  * tracing gc?

* tree sitter?

* good repl
  * lots of bugs right now
  * ? for help
  * keybind to exit
  * h to show history
  * autocompletion
  * multiline editing
  * some way to see variables defined in the repl
  * should use vm probably

* functions
* control flow
  * if/else
  * loops
  * match

* structs
* tuples
* enums
* traits
* generics

* modules

* performance
  * benchmark suite
  * compare jit vs vm vs interpreted
  * optimise mir
  * optimise jit codegen
  * optimise vm
  * look at hot paths and jit

* use an interner?
