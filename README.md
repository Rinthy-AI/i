i is a language for writing pure array-valued expressions

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
