from itensor import Tensor, Component as i

print(Tensor([1]))
print(Tensor([1, 2]))
print(Tensor([[1], [2]]))
print(Tensor([[1, 2]]))
print(Tensor([[1, 2], [3, 4]]))

print(i("ik*kj~ijk"))

