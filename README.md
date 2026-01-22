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

* get incremental compiling good
  * only recompile files which have changed
  * run each file in thread to parse
  * sync threads for module resolution and name resolution
  * run in parallel again for function level up to mir generation
  * persist to disk

* make runtime good
  * support heap based values
  * garbage collection

* lsp

* bugs
  * defining a variable should return unit type, not the value assigned
    * not sure if this is a bug yet since we don't have functions yet so ignore
  * hover for variables doesn't show type in blocks
  * infer has some stuff in there which is not related to type inference
  * string consts not working properly in jit
  * floats don't work with mir
    * should mir have type information from type check?

* improvements
  * rename int to Int, float to Float, bool to Bool, string to String
  * handle postfix operators in parser better
    * should be the same as prefix operators
  * is_bracket_index_not_list() uses speculative (re)lexing to check if [...] contains a comma (ie list) or not (ie index/slice)
    * maybe do this in a cleaner way?
    * feels weird to re lex
  * i think slices and range expressions are a bit weird and buggy
    * need to review
    * they should be the same
    * they should both produce a slice? type
    * is this valid 10..9 or error? 10..10?

* more helpful warnings
  * let/var statements are not in this language, omit and use := instead
  * doc comments which are not attached to anything
    * suggest attaching them to the next function/variable definition/doccomment
  * unused variables (naming convention: _varname to ignore warning)
  * unused functions (naming convention: _funcname to ignore warning)
  * shadowing variables
  * unreachable code

* review docs
  * make it read better
  * add more examples
  * unsure if i should make the docs have useful examples or just focus on language features

* cli
  * self update

* structs
* tuples
* enums
* traits / interfaces

* exceptions / error handling
  * try/catch
  * result type
  * option type

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
    * scripts can have use statements from std and pkg but nothing else. like uvx these are auto downloaded

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

* playground
* good repl
  * lots of bugs right now
  * ? for help
  * keybind to exit
  * h to show history
  * autocompletion
  * multiline editing
  * some way to see variables defined in the repl
  * should use vm probably
