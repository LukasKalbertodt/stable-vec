extern crate stable_vec;

use stable_vec::StableVec;


struct EchoDrop(pub char);

impl Drop for EchoDrop {
    fn drop(&mut self) {
        println!("I was dropped: {}", self.0);
    }
}

fn main() {
    let mut sv = StableVec::new();
    sv.push(EchoDrop('a'));
    let b = sv.push(EchoDrop('b'));
    sv.push(EchoDrop('c'));

    println!("--- removing 'b' ...");
    sv.remove(b);

    println!("--- going out of scope now ...");
}
