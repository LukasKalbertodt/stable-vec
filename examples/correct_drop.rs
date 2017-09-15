extern crate stable_vec;

use stable_vec::StableVec;

/// A dummy type which prints its character when dropped.
struct EchoDrop(pub char);

impl Drop for EchoDrop {
    fn drop(&mut self) {
        println!("I was dropped: {}", self.0);
    }
}

fn main() {
    let mut sv = StableVec::new();
    sv.push(EchoDrop('a'));
    let b_idx = sv.push(EchoDrop('b'));
    sv.push(EchoDrop('c'));

    {
        // Removing it from the vector shouldn't drop the value: it is moved
        // out the vector into this function.
        println!("--- removing 'b' (nothing should be dropped!) ...");
        let _b = sv.remove(b_idx);

        // But now the value goes out of scope and it should be dropped now.
        println!("--- letting 'b' go out of scope (it should be dropped now!) ...");
    }

    // The vector will be dropped at the end of this function and should drop
    // all elements inside it which haven't been removed yet ('a' and 'c').
    println!("--- Letting 'sv' go out of scope (it should drop 'a' and 'c'!) ...");
}
