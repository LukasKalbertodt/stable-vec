use stable_vec::StableVec;


fn main() {
    let mut sv = StableVec::from(&[0, 1, 2, 3, 4, 5]);
    sv.remove(1);
    sv.remove(4);

    for (i, e) in &sv {
        println!("{} -> {:?}", i, e);
    }

    println!("-------");
    for e in sv.values_mut() {
        *e += 1;
        println!("{:?}", e);
    }

    // StableVec implements `FromIterator`
    let sv: StableVec<_> = (1..9).collect();
    println!("{:?}", sv);
}
