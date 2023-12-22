# fxyt

Rust implementation of the [FXYT graphics description language](https://github.com/susam/fxyt). It's a cute weekend project to write a little recursive descent parser and interpreter for. The only public method is `render()`, which takes a string slice containing a FXYT program (see @susam's github linked earlier for syntax) and returns a Vec of 256x256 row-major RGB8 arrays with associated frame intervals. If the provided program references T in any way, there will be 256 frames in the Vec, otherwise there will be just one. There's less than 500 lines in the single .rs file, including a complete set of AST nodes and error types. The only dependencies are `rgb` and `thiserror`.
