#![feature(test)]
#[macro_use]
extern crate bencher;
use bencher::Bencher;
extern crate stable_vec;
extern crate test;

use stable_vec::StableVec;

fn clear(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..1000000 {
        v.push(x);
    }
    let sv = StableVec::from_vec(v);
    a.iter(|| {
        sv.clone().clear();
    });
}

//uses next on sv with large chunks of deleted elements
fn next_large_chunks(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..1000000 {
        v.push(x);
    }
    let mut sv = StableVec::from_vec(v);
    for x in 0..1000000 {
        if x < 25000 || (x >= 25001 && x < 100000) {
            sv.remove(x);
        }
    }
    a.iter(|| {
        let mut it = sv.iter();
        assert_eq!(it.next(), Some(&25000));
        assert_eq!(it.next(), Some(&100000));
    });
}

//next on sv with every 5th element deleted
fn next_5th_elem_del(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..1000000 {
        v.push(x);
    }
    let mut sv = StableVec::from_vec(v);

    for i in 0..1000000 {
        if i % 5 == 0 {
            sv.remove(i);
        }
    }

    a.iter(|| {
        let mut itr = sv.iter();
        while itr.next() != None {}
    });
}

//next on sv with all but 5th elements deleted
fn next_only_5th(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..10000 {
        v.push(x);
    }
    let mut sv = StableVec::from_vec(v);
    for i in 0..10000 {
        if i % 5 != 0 {
            sv.remove(i);
        }
    }

    a.iter(|| {
        let mut itr = sv.iter();
        while itr.next() != None {}
    });
}

//next on sv with no deleted elements
fn next_no_del(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..100000 {
        v.push(x);
    }
    let sv = StableVec::from_vec(v);
    a.iter(|| {
        let mut itr = sv.iter();
        while itr.next() != None {}
    });
}

fn delete(a: &mut Bencher) {
    let mut v = Vec::new();
    for x in 0..10000 {
        v.push(x);
    }
    let mut sv = StableVec::from_vec(v);
    a.iter(|| {
        for x in 0..10000 {
            if (x >= 250 && x < 300)
                || (x >= 400 && x < 500)
                || (x >= 2500 && x < 3000)
                || (x >= 4000 && x < 5000)
                || (x >= 6000 && x < 7000)
            {
                sv.remove(x);
            }
        }
    });
}

fn grow(a: &mut Bencher) {
    a.iter(|| {
        let mut v = Vec::new();
        for x in 0..10000 {
            v.push(x);
        }
        let mut sv = StableVec::from_vec(v);
        sv.grow(10000);
    });
}
benchmark_group!(
    benches,
    clear,
    next_large_chunks,
    next_5th_elem_del,
    next_only_5th,
    next_no_del,
    delete,
    grow
);
benchmark_main!(benches);
