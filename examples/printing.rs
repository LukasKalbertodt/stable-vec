use stable_vec::StableVec;


fn main() {
    let mut sv = StableVec::from(&['a', 'b', 'c', 'd', 'e', 'f']);
    println!("{:?}", sv);

    sv.remove(1);
    sv.remove(4);
    println!("{:?}", sv);

    sv.push('x');
    println!("{:?}", sv);
}
