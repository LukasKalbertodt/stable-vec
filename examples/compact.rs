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
    for i in 0..sv.next_push_index() {
        println!("{} -> {:?}", i, sv.get(i));
    }

    let n_before_compact = sv.num_elements();

    sv.make_compact();
    println!("--- after compact():");
    for i in 0..sv.next_push_index() {
        println!("{} -> {:?}", i, sv.get(i));
    }

    println!("compact:  {}", sv.is_compact());
    println!("n before: {}", n_before_compact);
    println!("n after:  {}", sv.num_elements());
}
