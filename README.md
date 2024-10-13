i is a language for writing pure array-valued expressions

The name is subject to change.

Dependency Expressions
---

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

Combinator Expressions
---

Aside from dependency expressions, i supports expression combinators. Currently
the only combinator implemented is compose, which works according to the
typically mathematical sense. For example, this matrix multiply expression
first applies `p` to the argument and then applies `a` to the result:

`mm: a . p`.

Notes on iteration domain inference
---

There is a 1-D iteration domain for each unique atomic (single-char) index. For
example, `ik*kj~ijk` has three domains, one for each of `i`, `j`, and `k`. Each
compound (multi-char) index must index a scalar in the input Array. Therefore,
both input Arrays in this expression are must be 2-D and the output Array must
be 3-D. The numerical values of the iteration domains are determined by
matching the atomic index with the size of the corresponding dimension in the
Array. For example, the `i` iteration domain goes from 0 to in0.shape[0], j
from 0 to in1.shape[1], and k from 0 to in0.shape[1] == in1.shape[0].  Notice
how the expression enforces the shape correspondance between `in0` and `in1`.
Any atomic indices in the output not informed by an input index go from 0 to 1
(that is, additional atomic indices in the output index can "unsqueeze" the
output). This determines the domains for all atomic indices and from these, the
shape of the output is determined. For example, in the above example, the
output is an Array of shape `[in0.shape[0], in1.shape[1], in0.shape[1]]`.
Whereas atomic indices present in the output but not the input signify
unsqueeze, atomic indices present in any input but not the output signify a
reduction (or a squeeze). This is handled accordingly: the output Array is
initialized with the identity of the given operation, e.g., 0 for
`Add`/`Accum`, 1 for `Mul`/`Product`. The an N-D iteration domain is
constructed every iteration of which computes some scalar quantity and updates
the corresponding element of the output.

Questions:
  - Can repeat indices in the inputs? What about the outputs?
    - I _think_ inputs is fine, outputs not, but have to think more.
