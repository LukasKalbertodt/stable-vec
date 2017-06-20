extern crate stable_vec;

use stable_vec::StableVec;


fn main() {
    let mut sv = StableVec::new();
    sv.push('a');
    let b = sv.push('b');
    let c = sv.push('c');
    sv.push('d');
    sv.push('e');
    let f = sv.push('f');
    sv.push('g');

    sv.remove(b);
    sv.remove(c);
    sv.remove(f);

    println!("--- before compact():");
    for i in 0..sv.next_index() {
        println!("{} -> {:?}", i, sv.get(i));
    }

    sv.compact();
    println!("--- after compact():");
    for i in 0..sv.next_index() {
        println!("{} -> {:?}", i, sv.get(i));
    }
}
