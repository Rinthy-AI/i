import time

import numpy as np
from itensor import Tensor, Component as i

np.random.seed(0)

n = 2
a = np.random.rand(n, n)
b = np.random.rand(n, n)

t = time.time()
c = a @ b
print(f"{time.time() - t} seconds")
print(c)

t = time.time()
out = (i("ik*kj~ijk") | i("+ijk~ij")).exec(
    Tensor(a.tolist()),
    Tensor(b.tolist()),
)
print(f"{time.time() - t} seconds")
print(np.array(out.data).reshape(out.shape))

print('all close? ', np.allclose(np.array(out.data).reshape((n, n)), c, rtol=1e-5, atol=1e-7))

