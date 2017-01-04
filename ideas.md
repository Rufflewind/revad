# Ideas

The existing design is fairly efficient but it lacks in flexibility.  Now, it would be nice to achieve these goals without sacrificing (too much) efficiency, but that may not be possible if we admit richer nodes into our graph (= allocation for each node! :/ ).  We also wanna keep the separation between tape and the filled in gradients, so we can re-use the tape.  This is difficult when you want arbitrarily-typed variable!

## Richly typed variables

We don't want to assume everything is an amorphous vector of floats or something, which means would encourage users to write things in a type-unsafe way (or unpack everything into dumb vectors).

Update: Maybe we can get away with a homogeneously typed tape, and then compose differently typed tapes together using structures (like how Iterators work)?  But how does a generic tape work?

## Better composability

It would be nice to allow nesting of some sort.  Right now, the whole process of AD-ing a function is monolithic: you can't embed a subgraph into a node.

## Reduce memory usage

It'd be nice to have some “manual override” that allows you to reduce memory usage for a specific portion of the function that is known to be intensive.

Also, it would be nice to implement the CTZ forgetting (and recomputation) strategy for a long repeated calculation, which I think is asymptotically optimal and can reduce both memory usage and recompute costs to logarithmic scaling.  It'd be even better to see how this pattern can be applied to more general recursive/iterative control flow.

## Prevent variables of different tapes from being used together

We can use [branded indices](https://github.com/bluss/indexing) for this.

## Abstract notion of adjoint functions

Here is how reverse-mode AD works on an abstract mathematical level.

Suppose we have a function:

    f : X -> Y

which maps each point `x` in the input space `X` to a point `y` in the output space `Y`.  If `f` is differentiable, then there exists an **adjoint** function:

    adj_f: (x : X) -> GY(f(x)) -> GX(x)

that computes the gradient `gx: GX(x)` of `f` at the point `x : X` along the cotangent vector `gy: GY(f(x))`.  Mathematically, `GX(x)` is the cotangent space of `X` at the point `x` (and similarly for `GY`).  It is a vector space composed of every possible differential:

    {dx₁, dx₂, dx₃, …, plus all linear combinations of such}

i.e. differentials are the basis vectors.  Don't confuse this with the “differentials” in forward-mode AD, which are really tangent vectors!

   - For a tangent vector, the coefficients are differentials (`dx/dt`), whereas the basis vectors are directional derivatives (`∂/∂x`).
   - For a cotangent vector, the coefficients are directional derivatives (`∂t/∂x`), whereas the basis vectors are differentials (`dx`).

Notice that `GX` is parametrized by `x : X`.  We can't encode this in Rust though, so we will instead pretend that `GX` is independent of `x : X`.
