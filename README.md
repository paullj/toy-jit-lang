**toy**

toy is a simple programming language that is made for fun.

**goals**

* easy to learn
* helpful errors
* performant (not the case right now)
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
  * unreachable code

* clean up
  * maybe make cli just a cli and move all the logic elsewhere

* functions
* closures

* control flow
  * loops
    * while
    * for x in y
    * loop

* garbage collection
  * reference counting?
  * tracing gc?
  * hybrid?

* structs
* tuples
* enums
* traits / interfaces

* pattern matching

* generics

* modules
  * import / export system
  * public / private visibility
  * scripts / application / libraries
    * scripts are single file programs
      * can be run directly without a build step
    * applications are multi file programs with a main entry point
      * can be built into an executable
    * libraries are multi file programs without a main entry point
      * can be built into a library file to be used by other programs
      * can be published to git repo

  * build system
    * portable build with embedded interpreter/jit
    * cross-compilation support

* extern functions / ffi
  * ability to call functions from other languages (e.g. c, rust)
  * define functions in other languages and call them from toy

* std lib
  * basic i/o
    * stdin
    * stdout
    * stderr
  * string manipulation
  * result / option types
  * type conversion
  * math functions
  * date/time functions
  * random
  * collections
    * arrays
    * hash maps
    * sets
  * file i/o
  * networking

* concurrency
  * async/await
  * threads
  * channels

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

* performance
  * benchmark suite
  * compare jit vs vm vs interpreted
  * optimise mir
  * optimise jit codegen
  * optimise vm
  * look at hot paths and jit
  * use an interner?
