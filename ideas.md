# Ideas

The existing design is fairly efficient but it lacks in flexibility.  Now, it would be nice to achieve these goals without sacrificing (too much) efficiency, but that may not be possible if we admit richer nodes into our graph (= allocation for each node! :/ ).  We also wanna keep the separation between tape and the filled in gradients, so we can re-use the tape.  This is difficult when you want arbitrarily-typed variable!

## Richly typed variables

We don't want to assume everything is an amorphous vector of floats or something, which means would encourage users to write things in a type-unsafe way (or unpack everything into dumb vectors).

## Better composability

It would be nice to allow nesting of some sort.  Right now, the whole process of AD-ing a function is monolithic: you can't embed a subgraph into a node.

## Reduce memory usage

It'd be nice to have some “manual override” that allows you to reduce memory usage for a specific portion of the function that is known to be intensive.

Also, it would be nice to implement the CTZ forgetting (and recomputation) strategy for a long repeated calculation, which I think is asymptotically optimal and can reduce both memory usage and recompute costs to logarithmic scaling.  It'd be even better to see how this pattern can be applied to more general recursive/iterative control flow.

## Prevent variables of different tapes from being used together

We can use [branded indices](https://github.com/bluss/indexing) for this.
