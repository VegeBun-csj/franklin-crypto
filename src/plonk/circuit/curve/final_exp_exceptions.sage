from enum import Enum

class Ops(Enum):
    ExpByX = 0,
    Mul = 1,
    Square = 2,
    Conj = 3,
    Frob = 4
    
class X:
    def __init__(self, op, out, first_in, second_in):
        self.op = op
        self.out = out
        self.first_in = first_in
        self.second_in = second_in

p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
x = 4965661367192848881
r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
d = p^4 - p^2 + 1
# for Fuentes Castaneda we should additionally raise the result to the power
m = 2 * x * (6*x^2 + 3 * x + 1)
assert(d % r == 0)

class Bn256HardPartMethod(Enum):
    Devegili = 0,
    FuentesCastaneda = 1,
    Naive = 2

hard_part_method = Bn256HardPartMethod.FuentesCastaneda


if hard_part_method == Bn256HardPartMethod.Devegili:
    (f, f2, a, b, tmp, t0, t1, nil) = (0, 1, 2, 3, 4, 5, 6, 7)
    num_of_vars = 7
    ops_chain = [
        X(Ops.ExpByX, a, f, nil), X(Ops.Square, b, a, nil), X(Ops.Square, f2, f, nil), X(Ops.Mul, a, b, f2),
        X(Ops.Square, a, a, nil), X(Ops.Mul, a, a, b), X(Ops.Mul, a, a, f), X(Ops.Conj,a, a, nil), X(Ops.Frob, b, a, 1),
        X(Ops.Mul, b, a, b), X(Ops.Mul, a, a, b), X(Ops.Frob, t0, f, 1), X(Ops.Mul, t1, t0, f), 
        X(Ops.Square, tmp, t1, nil), X(Ops.Square, tmp, tmp, nil), X(Ops.Square, tmp, tmp, nil), 
        X(Ops.Mul, t1, tmp, t1), X(Ops.Mul, a, t1, a), X(Ops.Square, t1, f2, nil), X(Ops.Mul, a, a, t1), 
        X(Ops.Square, t0, t0, nil), X(Ops.Mul, b, b, t0), X(Ops.Frob, t0, f, 2), X(Ops.Mul, b, b, t0), 
        X(Ops.ExpByX, t0, b, nil), X(Ops.Square, t1, t0, nil), X(Ops.Square, t0, t1, nil), X(Ops.Mul, t0, t0, t1), 
        X(Ops.ExpByX, t0, t0, nil), X(Ops.Mul, t0, t0, b), X(Ops.Mul, a, t0, a), X(Ops.Frob, t0, f, 3), 
        X(Ops.Mul, f, t0, a)
    ]
elif hard_part_method == Bn256HardPartMethod.FuentesCastaneda:
    (f, a, b, tmp, t, nil) = (0, 1, 2, 3, 4, 5)
    num_of_vars = 5
    ops_chain = [
        X(Ops.ExpByX,a, f, nil), X(Ops.Square, a, a, nil), X(Ops.Square, b, a, nil), X(Ops.Mul, b, a, b), 
        X(Ops.ExpByX, t, b, nil), X(Ops.Conj, tmp, f, nil), X(Ops.Frob, tmp, tmp, 3), X(Ops.Mul, f, f, tmp),
        X(Ops.Mul, f, f, t), X(Ops.Mul, b, b, t), X(Ops.Square, t, t, nil), X(Ops.ExpByX, t, t, nil), 
        X(Ops.Mul, b, b, t), X(Ops.Conj, tmp, a, nil), X(Ops.Mul, t, b, tmp), X(Ops.Frob, tmp, t, 3), 
        X(Ops.Mul, f, f, tmp), X(Ops.Frob, tmp, t, 1), X(Ops.Mul, f, f, tmp), X(Ops.Mul, f, f, b), 
        X(Ops.Frob, tmp, b, 2), X(Ops.Mul, f, f, tmp)
    ]
else:
    (f, fp, tmp, fp2, fp3, fu, fu2, fu3, y3, fu2p, fu3p, y2, y0, y1, y4, y5, y6, t0, t1, nil) = (
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
    )
    num_of_vars = 19
    ops_chain = [
        X(Ops.Frob, fp, f, 1), X(Ops.Frob, fp2, f, 2), X(Ops.Frob, fp3, fp2, 1), X(Ops.ExpByX, fu, f, nil), 
        X(Ops.ExpByX, fu2, fu, nil), X(Ops.ExpByX, fu3, fu2, nil), X(Ops.Frob, tmp, fu, 1), X(Ops.Conj, y3, tmp, nil), 
        X(Ops.Frob, fu2p, fu2, 1), X(Ops.Frob, fu3p, fu3, 1), X(Ops.Frob, y2, fu2, 2), X(Ops.Mul, tmp, fp, fp2), 
        X(Ops.Mul, y0, tmp, fp3), X(Ops.Conj, y1, f, nil), X(Ops.Conj, y5, fu2, nil), X(Ops.Mul, tmp, fu, fu2p), 
        X(Ops.Conj, y4, tmp, nil), X(Ops.Mul, tmp, fu3, fu3p), X(Ops.Conj, tmp, tmp, nil), X(Ops.Square, tmp, tmp, nil),
        X(Ops.Mul, tmp, tmp, y4), X(Ops.Mul, y6, tmp, y5), X(Ops.Mul, tmp, y3, y5), X(Ops.Mul, t1, tmp, y6), 
        X(Ops.Mul, y6, y2, y6), X(Ops.Square, t1, t1, nil), X(Ops.Mul, t1, t1, y6), X(Ops.Square, t1, t1, nil), 
        X(Ops.Mul, t0, t1, y1), X(Ops.Mul, t1, t1, y0), X(Ops.Square, t0, t0, nil), X(Ops.Mul, f, t0, t1)
    ]
    
        
def check():
    scratchpad = [0 for i in xrange(num_of_vars)]
    scratchpad[0] = 1
    num_ops = len(ops_chain)
    
    for (idx, item) in enumerate(ops_chain):
        out = item.out
        if item.op == Ops.ExpByX:
            scratchpad[out] = scratchpad[item.first_in] * x
        elif item.op == Ops.Mul:
            scratchpad[out] = scratchpad[item.first_in] + scratchpad[item.second_in]
        elif item.op == Ops.Square:
            scratchpad[out] = scratchpad[item.first_in] * 2
        elif item.op == Ops.Conj:
            scratchpad[out] = -scratchpad[item.first_in]
        else:
            scratchpad[out] = scratchpad[item.first_in] * p^item.second_in
        
        if d % scratchpad[out] == 0 and scratchpad[out] != -1:
            print idx, "error"
    
    actual_result = d/r
    if hard_part_method == Bn256HardPartMethod.FuentesCastaneda:
        actual_result *= m
    assert(scratchpad[0] == actual_result)
    print "success"
    

check()

