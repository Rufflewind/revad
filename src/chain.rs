//! This module implements explicit algorithms for handling loops in
//! reverse-mode AD code.
//!
//! `FullChain` is the naive approach: it basically records a snapshot at
//! every step.  A smarter way is done by the `CtzChain`, which will keep only
//! a small subset of the snapshot (roughly, a logarithmic amount) at the
//! expense of requiring recomputations (also, a logarithmic amount).
//!
//! Both of these chains are constructed from an iterator of *snapshots*,
//! which are arbitrary objects that may be used to compute the adjoints.  To
//! compute an adjoint value, the adjoint function `j: J` (conceptually, the
//! transposed Jacobian) is called in the following manner:
//!
//! ~~~text
//! let gx = j(s, gy);
//! ~~~
//!
//! where `s: S` is the snapshot and `gx: G` and `gy: G` are the adjoint
//! values.  Unfortunately the `sweep_*` methods have slightly differing
//! requirements on the signature of the adjoint function `J`: they differ in
//! how the snapshot is to be passed in (immutable/mutable/by value).  (I wish
//! there was a way to avoid this.)
//!
//! When using a loop to compute `f(f(f(…)))`, we use a condensed version of
//! `f` called the restoration function (`restore`) to recompute the
//! snapshots, if necessary.  A trivial definition of the restoration function
//! is given by `|x| f(x)`, if we assume the snapshots are just the input
//! values themselves.  But you may not want to use the trivial definition, if
//! the restoration function can be implemented in a simpler way (as is the
//! case if `f` is a linear function or is partly linear, in which case the
//! derivative would be independent or partly independent of the input
//! values).  In any case, the restoration function `r` must satisfy the
//! following condition with respect to the sequence of snapshots: if the
//! snapshot `s2` follows `s1`, then `r(s1)` must return `s2`.
//!
//! Pictorially, a chain is used to model the following control flow:
//!
//! ~~~text
//! x0 --+-> x1 --+-> x2 --+-> x3   (plain values)
//!      |        |        |
//!      |        |        |
//!      v        v        v
//!      s0       s1       s2       (snapshots)
//!      |        |        |
//!      |j       |j       |j       (adjoint function)
//!      v        v        v
//! g0 <-+-- g1 <-+-- g2 <-+-- g3   (adjoint values)
//!    ---->    ---->    ---->
//!      r        r        r        (restoration function)
//! ~~~
//!
//! The iterator is what produces the snapshots, so the chain implementations
//! never actually see the input values `x: X` directly.

/// Reifies an operation built from a loop.
///
/// When called as a function, it will run the adjoint functions in reverse.
pub struct FullChain<S, J> {
    snapshots: Vec<S>,
    adjoint: J,
}

impl<S, J> FullChain<S, J> {

    pub fn new<I>(snapshots: I, adjoint: J) -> Self where I: Iterator<Item=S> {
        FullChain {
            snapshots: snapshots.collect(),
            adjoint: adjoint,
        }
    }

    pub fn sweep<G>(&self, mut x: G) -> G where J: Fn(&S, G) -> G {
        for g in self.snapshots.iter().rev() {
            x = (self.adjoint)(g, x);
        }
        return x;
    }

    pub fn sweep_mut<G>(&mut self, mut x: G) -> G where J: FnMut(&mut S, G) -> G {
        for g in self.snapshots.iter_mut().rev() {
            x = (self.adjoint)(g, x);
        }
        return x;
    }

    pub fn sweep_once<G>(mut self, mut x: G) -> G where J: FnMut(S, G) -> G {
        loop {
            match self.snapshots.pop() {
                Some(g) => x = (self.adjoint)(g, x),
                None => return x,
            }
        }
    }
}

/// Lossily extend a vector of elements using an eviction strategy based on
/// the count-trailing-zeros operation.  The algorithm tends to evict items
/// further away from the current item.  The latest item is never evicted, and
/// existing items are left untouched.  Given `n` new items to be added, the
/// number of items that will actually get added to the vector is exactly
/// `ceil(log2(n)) + 1`.
pub fn ctz_extend<T, I>(v: &mut Vec<(usize, T)>, i0: usize, xs: I)
    where I: Iterator<Item=T> {
    let mut ruler: usize = 1;
    let mut ruler_max = -2;
    let start = v.len();
    for (i, x) in xs.enumerate() {
        v.push((i0 + i, x));
        let j = ruler_max - ruler.trailing_zeros() as i64;
        if j <= 0 {
            ruler = 1;
            ruler_max += 1;
        } else {
            ruler += 1;
            v.remove(start + (j as usize));
        }
    }
}

/// Maintains a partial tape using the count-trailing-zeros (CTZ) eviction
/// strategy.  This results in space usage that is logarithmic in the number
/// of steps, while also keeping a logarithmic amount of recomputation time.
///
/// FIXME: `sweep_mut` is not yet implemented.
pub struct CtzChain<S, J, R> {
    snapshots: Vec<(usize, S)>,
    adjoint: J,
    restore: R,
}

impl<S, J, R> CtzChain<S, J, R> {
    pub fn new<I>(snapshots: I, adjoint: J, restore: R) -> Self
        where I: Iterator<Item=S> {
        let mut chain = CtzChain {
            snapshots: Vec::new(),
            adjoint: adjoint,
            restore: restore,
        };
        ctz_extend(&mut chain.snapshots, 0, snapshots);
        chain
    }

    pub fn sweep<G>(&self, mut x: G) -> G
        where J: Fn(&S, G) -> G, R: Fn(&S) -> S {

        fn extend<S, R>(restored_snapshots: &mut Vec<(usize, S)>,
                        mut num_missing: usize, i: usize,
                        s_old: &mut Option<S>, restore: &R)
            where R: Fn(&S) -> S {
            ctz_extend(restored_snapshots, i + 1, Generator(|| {
                let mut s_new = match s_old {
                    &mut None => {
                        None
                    },
                    &mut Some(ref mut s_old) => {
                        num_missing -= 1;
                        if num_missing == 0 {
                            None
                        } else {
                            Some(restore(s_old))
                        }
                    },
                };
                ::std::mem::swap(s_old, &mut s_new);
                s_new
            }));
        }

        let mut j = match self.snapshots.last() {
            None => return x,
            Some(&(i, _)) => i + 1,
        };
        let mut restored_snapshots = Vec::new();
        for &(i, ref s) in self.snapshots.iter().rev() {
            loop {
                match restored_snapshots.pop() {
                    Some((i, s)) => {
                        let num_missing = j - i - 1;
                        if num_missing != 0 {
                            let mut s_old = Some((self.restore)(&s));
                            restored_snapshots.push((i, s));
                            extend(&mut restored_snapshots, num_missing,
                                   i, &mut s_old, &self.restore);
                        } else {
                            x = (self.adjoint)(&s, x);
                            j -= 1;
                        }
                    },
                    None => {
                        let num_missing = j - i - 1;
                        if num_missing != 0 {
                            let mut s_old = Some((self.restore)(&s));
                            extend(&mut restored_snapshots, num_missing,
                                   i, &mut s_old, &self.restore);
                        } else {
                            x = (self.adjoint)(&s, x);
                            j -= 1;
                            break;
                        }
                    },
                }
            }
        }
        x
    }

    pub fn sweep_once<G>(mut self, mut x: G) -> G
        where J: FnMut(S, G) -> G, R: FnMut(&S) -> S {
        let mut i = match self.snapshots.last() {
            None => return x,
            Some(&(j, _)) => j + 1,
        };
        loop {
            match self.snapshots.pop() {
                Some((j, s)) => {
                    let mut num_missing = i - j - 1;
                    if num_missing == 0 {
                        x = (self.adjoint)(s, x);
                        i -= 1;
                    } else {
                        let mut s_old = Some((self.restore)(&s));
                        self.snapshots.push((j, s));
                        let mut restore = self.restore;
                        ctz_extend(&mut self.snapshots, j + 1, Generator(|| {
                            let mut s_new = match &mut s_old {
                                &mut None => {
                                    None
                                },
                                &mut Some(ref mut s_old) => {
                                    num_missing -= 1;
                                    if num_missing == 0 {
                                        None
                                    } else {
                                        Some(restore(s_old))
                                    }
                                },
                            };
                            ::std::mem::swap(&mut s_old, &mut s_new);
                            s_new
                        }));
                        self.restore = restore;
                    }
                },
                None => {
                    assert_eq!(i, 0);
                    return x;
                },
            }
        }
    }
}

/// Wrap a `next` function into an `Iterator`.
pub struct Generator<F>(pub F);

impl<F, T> Iterator for Generator<F> where F: FnMut() -> Option<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        (self.0)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static E: f64 = 1.01;
    static N: i32 = 100;
    static X0: f64 = 4.2;

    fn f(mut x: Vec<f64>) -> Vec<f64> {
        x[0] = x[0].powf(E);
        x
    }

    #[test]
    fn full_chain() {
        let x0 = vec![X0];
        let expected = E.powi(N) * X0.powf(E.powi(N) - 1.0);

        let g = {
            let mut x = x0.clone();
            let mut i = 0;
            FullChain::new(Generator(|| {
                if !(i < N) {
                    return None;
                }
                i += 1;
                let mut x2 = f(x.clone());
                ::std::mem::swap(&mut x2, &mut x);
                Some(x2)
            }), |x: &Vec<f64>, mut g: Vec<f64>| {
                g[0] *= E * x[0].powf(E - 1.0);
                g
            })
        }.sweep(vec![1.0]);
        assert!((g[0] - expected).abs() < 1e-10);

        let g = {
            let mut x = x0.clone();
            let mut i = 0;
            FullChain::new(Generator(|| {
                if !(i < N) {
                    return None;
                }
                i += 1;
                let mut x2 = f(x.clone());
                ::std::mem::swap(&mut x2, &mut x);
                Some(x2)
            }), |x: &mut Vec<f64>, mut g: Vec<f64>| {
                g[0] *= E * x[0].powf(E - 1.0);
                g
            })
        }.sweep_mut(vec![1.0]);
        assert!((g[0] - expected).abs() < 1e-10);

        let g = {
            let mut x = x0.clone();
            let mut i = 0;
            FullChain::new(Generator(|| {
                if !(i < N) {
                    return None;
                }
                i += 1;
                let mut x2 = f(x.clone());
                ::std::mem::swap(&mut x2, &mut x);
                Some(x2)
            }), |x: Vec<f64>, mut g: Vec<f64>| {
                g[0] *= E * x[0].powf(E - 1.0);
                g
            })
        }.sweep_once(vec![1.0]);
        assert!((g[0] - expected).abs() < 1e-10);
    }

    #[test]
    fn ctz_chain() {
        let x0 = vec![X0];
        let expected = E.powi(N) * X0.powf(E.powi(N) - 1.0);

        let g = {
            let mut x = x0.clone();
            let mut i = 0;
            CtzChain::new(Generator(|| {
                if !(i < N) {
                    return None;
                }
                i += 1;
                let mut x2 = f(x.clone());
                ::std::mem::swap(&mut x2, &mut x);
                Some(x2)
            }), |x: &Vec<f64>, mut g: Vec<f64>| {
                g[0] *= E * x[0].powf(E - 1.0);
                g
            }, |x: &Vec<f64>| f(x.clone()))
        }.sweep(vec![1.0]);
        assert!((g[0] - expected).abs() < 1e-10);

        let g = {
            let mut x = x0.clone();
            let mut i = 0;
            CtzChain::new(Generator(|| {
                if !(i < N) {
                    return None;
                }
                i += 1;
                let mut x2 = f(x.clone());
                ::std::mem::swap(&mut x2, &mut x);
                Some(x2)
            }), |x: Vec<f64>, mut g: Vec<f64>| {
                g[0] *= E * x[0].powf(E - 1.0);
                g
            }, |x: &Vec<f64>| f(x.clone()))
        }.sweep_once(vec![1.0]);
        assert!((g[0] - expected).abs() < 1e-10);
    }
}
