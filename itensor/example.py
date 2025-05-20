from itensor import Tensor, Component as i

print(Tensor([1]))
print(Tensor([1, 2]))
print(Tensor([[1], [2]]))
print(Tensor([[1, 2]]))
print(Tensor([[1, 2], [3, 4]]))

#print(i("ik*kj~ijk"))
out = i("""
h: ik*kj~ijk
a: +ijk~ij
h.a
""").exec(
    Tensor([[1, 2], [3, 4]]),
    Tensor([[1, 2], [3, 4]]),
)

print(out)

