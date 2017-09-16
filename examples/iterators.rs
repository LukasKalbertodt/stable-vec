extern crate stable_vec;

use stable_vec::StableVec;



fn main() {
    let mut sv = StableVec::from(&[0, 1, 2, 3, 4, 5]);
    sv.remove(1);
    sv.remove(4);

    for e in &sv {
        println!("{:?}", e);
    }

    println!("-------");
    for e in &mut sv {
        *e += 1;
        println!("{:?}", e);
    }

    println!("-------");
    for e in &sv {
        println!("{:?}", e);
    }

    // StableVec implements `FromIterator`
    let sv: StableVec<_> = (1..9).collect();
    println!("{:?}", sv);
}
