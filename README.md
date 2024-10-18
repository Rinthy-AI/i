i is a language for writing pure array-valued expressions

The name is subject to change.

i can be embedded into Rust. Here is an example showing a matrix multiply:

``` rust
// from example/src/main.rs

let m = i!(m: ik*kj~ijk); // the multiplies of a matmul
let a = i!(a: +ijk~ij); // the accumulation of a matmul

let result = a(m(x, y)); // for some matrices `x` and `y`
```

# Status

- [x] parser
- [x] simple Rust backend that generates everything but combinators
- [x] proc macro `i!()` for writing/running i code directly in Rust

# Language Design

### Dependency Expressions

The fundamental expression type in i is the dependency expression.

Here is an example:

`p: ik*kj~ijk`.

Specifically, this expression `p` describes the multiply operations in a matrix
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

Dependency expression are so called because they map the dependencies of their
constituent scalar operations. All of the individual scalar operations indexed
by a dependency expression are completely independent of each other, meaning
they can be executed in any order. The expression are declarative in the sense
that they describe the dependencies without imposing any ordering or other
details about their execution. This property is a fundamental design motivation
of i. This allows for calculating the dependency relationship of any two scalar
ops by way of a simple recursive algorithm, eschewing the need to for the
read/write dependency analysis algorithms common in polyhedral compilers.

In addition to the _binary_ dependency expression above, there are also _unary_
expressions. An example is the accumulation portion of a matrix multiplication:

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

Finally, there are `no-op` dependency expressions which are purely for the
purpose of reshape/views on the inputs. An example is transpose:

`t: ij~ji`.

### Combinator Expressions

Aside from dependency expressions, i supports expression combinators. Currently
the only combinator implemented is compose, which works according to the
typically mathematical sense. For example, this matrix multiply expression
first applies `p` to the argument and then applies `a` to the result:

`mm: a . p`.

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

