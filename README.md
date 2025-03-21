<img src="logo.svg" alt="i logo" width="32">

---

i is a language for writing pure array-valued expressions.

i can be embedded into Rust. Here is an example showing a matrix multiply:

``` rust
// from example/src/main.rs

// matrix multiplication, multiplies, accumulation, expression chaining
let mm = i!(
    m: ik*kj~ijk
    a: +ijk~ij
    m.a
);

let result = mm(x, y); // for some matrices `x` and `y`
```

# Status

- [x] parser
- [x] basic (naive) Rust backend
- [x] proc macro `i!()` for writing/running i code directly in Rust

# Language Design

### Index Expressions

The fundamental expression type in i is the index expression.

Here is an example:

`m: ik*kj~ijk`.

Specifically, this expression `m` describes the multiply operations in a matrix
multiplication (without the accumulations). It is read as "ik times kj gives
ijk". `ik` and `kj` are 2-dimensional indices over the arguments to the
expression and `ijk` is a 3-dimensional index over the resulting array.

The domains of the indices are determined at runtime by the dimensions of the
arguments. In this case, `i` indexes the 0 dimension of the left input, `k`
indexes the 1 dimension of the left input _and_ the 0 dimension of the right
input, and `j` indexes the 1 dimension of the right input. Repeated indices
enforce shape constraints. In this case, the familiar constraint of matrix
multiplication that the number of columns of the left matrix must equal the
number of rows of the right matrix.

The full domain of the function is determined by the Cartesian product of the
domains of all unique indices. That is, there is one operation performed for
every (i,j,k) triple.

Index expression are so called because they map the dependencies of their
constituent scalar operations. All of the individual scalar operations indexed
by an index expression are completely independent of each other, meaning they
can be executed in any order. The expression are declarative in the sense that
they describe the dependencies without imposing any ordering or other details
about their execution. This property is a fundamental design motivation of i.
This allows for calculating the dependency relationship of any two scalar ops
by way of a simple recursive algorithm, eschewing the need to for the
read/write dependency analysis algorithms common in polyhedral compilers.

In addition to the _binary_ index expression above, there are also _unary_
index expressions. An example is the accumulation portion of a matrix
multiplication:

`a: +ijk~ij`.

This expression describes a sum over a 3-dimensional array resulting in a
2-dimensional array. Again, the index domains are determined by the input
shapes. The `k` index being present on the left but absent from the right
indicates that `k` indexes an axis of reduction. That is, the sum operates over
the 2 dimension of the input array and is not represented in the resulting
array.

Conversely, indices present on the right but not the left indicate
"unsqueezing" where an additional dimension of size 1 is added to the output.
For example: `i~ij`.

Finally, there are `no-op` index expressions which are purely for the purpose
of reshape/views on the inputs. An example is transpose:

`t: ij~ji`.

### Combinator Expressions

Aside from index expressions, i supports expression combinators. Currently the
only combinator implemented is chain, which passes the output of one expression
to the input of another.  For example, this matrix multiply expression first
applies `m` to the argument and then applies `a` to the result:

`mm: m.a`.

### Open Design Questions

- What does a repeated index in a single argument array indicate?
  - It seems the most natural interpretation of this based on the description
    of the algorithm domain above is that it results in a single domain the
    indexes the diagonal.
    - If this is the case, how could you enforce inter-array size constraints,
      e.g., square matrix?
- What does a repeated index in the resulting array indicate?
  - Following the logic above, the natural interpretation seems to be that it
    would index the diagonal. For example, `i~ii` would return a 2-D array
    where the diagonal holds the original 1-D input. Then what would the
    off-diagonal elements be? 0 seems obvious, but is there a reason this
    should be true?
- What other combinators make sense to add?
- How to tell with things like ReLU? Typically this is implemented as
  `max(x,0)`, but we don't have `0`. We don't have `max` either, but that's a
  smaller decision than adding numbers. One idea is introducing some small
  number of built-in functions like 0 which returns a 0s array of the
  appropriate shape (what if the shape cannot be inferred?).
- How do we handle multiple uses of the same input? For example, normalize:
  `x/x.sum()`. Maybe a "repeater" combinator that repeats its input?
- In general how do we handle expressions of multiple inputs? Haskell has
  currying. Maybe that could be useful here?
- How can we support stride iteration?
- How could we do a 3x3 box filter (the example from the Halide paper)?
- How could we do histogram? Do we even care about this?
- In general, reductions are order-dependent, but currently we ignore this and
  only consider associative reductions. Should we support non-associative
  reductions?
- Should i be intrinsically affine indexed? That is, can the backend
  interfacing code be written to expect single-usize indexing?

## Inspiration
- [FlashAttention](https://arxiv.org/pdf/2205.14135) (hmm, could a compiler
  learn/find FlashAttention?)
- [TensorComprehensions](https://arxiv.org/pdf/1802.04730) (wow, terse DSLs are
  cool af.)
- [Torch einsum](https://pytorch.org/docs/stable/generated/torch.einsum.html)
  (damn, even more terse than TCs.)
- [Halide](https://people.csail.mit.edu/jrk/halide-pldi13.pdf) (decouple alg
  description from scheduling, search for fast kernels.)
- [tinygrad](https://github.com/tinygrad/tinygrad) (simple good, search for
  fast kernels.)

