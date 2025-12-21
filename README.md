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
  * token span sometimes includes whitespace and trivia before the token
  * hover for variables doesn't work in blocks
  * goto definition for variables in blocks doesn't work
  * find references for variables in blocks doesn't work
  * lsp should support comments better
    * hover over function/variable def shows comment
    * cmd + / ctrl + / to toggle comment on line or selection

* more helpful errors
  * if you use semicolons as line terminators, give helpful error messages saying not needed
  * undefined variables which are similar to defined ones
  * undefined variables in a nearby scope - say that is not in scope and show where they are defined
  * trying to use +. -. *. /. on two ints (give suggestion to use +, -, *, /)
  * trying to use + - * / on two floats (give suggestion to use +., -., *., /.)
  * trying to use x = 1 before x is defined, suggest using := to define
  * trying to divide by zero (give warning at compile time if possible)
  * trying to define an int or float larger than the type can hold (give warning at compile time)
  * incomplete expressions (e.g. "x + " at end of file, should say did you forget another operand?)
  * assigning an empty block to a variable (should say did you mean to have an expression in the block?)


* more helpful warnings
  * unused variables (naming convention: _varname to ignore warning)
  * unused functions (naming convention: _funcname to ignore warning)
  * shadowing variables
  * [not yet because all code is reachable] unreachable code


* garbage collection
  * reference counting?
  * tracing gc?

* tree sitter?

* mir - almost 1:1 to machine code but optimise
* register based vm
* look at hot paths and jit

* performance

* good repl
  * lots of bugs right now
  * ? for help
  * keybind to exit
  * h to show history
  * autocompletion
  * multiline editing
  * some way to see variables defined in the repl
  * should use vm probably

* blocks
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

* use an interner?
