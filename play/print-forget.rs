// print a pretty picture that shows which elements are cached and which are
// forgotten
//
// here, we use a simple CTZ-based strategy (which I believe is optimal); the
// count-trailing-zero (CTZ) operation is used to generates the so-called
// ruler sequence, which is controls how long each new cache value can live
// for (i.e. the height of each column in the diagram produced by this
// program); funny enough, CTZ *also* appears in the formula that determines
// which index of the existing cache vector to annihilate so as to lead to the
// same desired result.
//
// anyway, number trickery aside, this strategy has the property of
// eliminating the caches that are further away from the "current" entry;
// the result looks like the ticks on a log2-scale axis of a plot

fn main() {

    let mut l = Vec::new();

    let mut ruler: usize = 1;
    let mut ruler_max = -2;
    for i in 0 .. 64 {
        l.push(i);

        let j = ruler_max - ruler.trailing_zeros() as i64;
        if j <= 0 {
            ruler = 1;
            ruler_max += 1;
        } else {
            ruler += 1;
            l.remove(j as usize);
        }

        for u in 0 .. i + 1 {
            if l.contains(&u) {
                print!("x");
            } else {
                print!(" ");
            }
        }
        println!("");
    }

}
