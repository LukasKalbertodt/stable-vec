use super::StableVec;

quickcheck! {
    fn reordering_compact(insertions: u16, to_delete: Vec<u16>) -> bool {
        let insertions = insertions + 1;
        // Create stable vector containing `insertions` zeros. Afterwards, we
        // remove at most half of those elements
        let mut sv = StableVec::from(vec![0; insertions as usize]);
        for i in to_delete {
            let i = (i % insertions) as usize;
            if sv.has_element_at(i) {
                sv.remove(i);
            }
        }

        // Remember the number of elements before and call compact.
        let sv_before = sv.clone();
        let n_before_compact = sv.num_elements();
        sv.reordering_make_compact();

        n_before_compact == sv.num_elements()
            && sv.is_compact()
            && (0..n_before_compact).all(|i| sv.get(i).is_some())
            && sv_before.iter().all(|e| sv.contains(e))
    }

    fn compact(insertions: u16, to_delete: Vec<u16>) -> bool {
        let insertions = insertions + 1;
        // Create stable vector containing `insertions` zeros. Afterwards, we
        // remove at most half of those elements
        let mut sv = StableVec::from(vec![0; insertions as usize]);
        for i in to_delete {
            let i = (i % insertions) as usize;
            if sv.has_element_at(i) {
                sv.remove(i);
            }
        }

        // Remember the number of elements before and call compact.
        let sv_before = sv.clone();
        let items_before: Vec<_> = sv_before.iter().cloned().collect();
        let n_before_compact = sv.num_elements();
        sv.make_compact();


        n_before_compact == sv.num_elements()
            && sv.is_compact()
            && (0..n_before_compact).all(|i| sv.get(i).is_some())
            && sv == items_before
    }

    fn from_and_extend_and_from_iter(items: Vec<u8>) -> bool {
        use std::iter::FromIterator;

        let iter_a = items.iter().cloned();
        let iter_b = items.iter().cloned();

        let sv_a = StableVec::from_iter(iter_a);
        let sv_b = {
            let mut sv = StableVec::new();
            sv.extend(iter_b);
            sv
        };
        let sv_c = StableVec::from(&items);

        sv_a.num_elements() == items.len()
            && sv_a == sv_b
            && sv_a == sv_c
    }
}

#[test]
fn compact_tiny() {
    let mut sv = StableVec::from(&[1.0, 2.0, 3.0]);
    assert!(sv.is_compact());

    sv.remove(1);
    assert!(!sv.is_compact());

    sv.make_compact();
    assert_eq!(sv.into_vec(), &[1.0, 3.0]);
}

#[test]
fn insert_into_hole_and_grow() {
    let mut sv = StableVec::from(&['a', 'b']);
    sv.reserve(10);

    assert_eq!(sv.num_elements(), 2);
    assert_eq!(sv.insert_into_hole(0, 'c'), Err('c'));
    assert_eq!(sv.insert_into_hole(1, 'c'), Err('c'));
    assert_eq!(sv.insert_into_hole(2, 'c'), Err('c'));

    sv.remove(1);

    assert_eq!(sv.num_elements(), 1);
    assert_eq!(sv.insert_into_hole(0, 'c'), Err('c'));

    assert_eq!(sv.insert_into_hole(1, 'c'), Ok(()));
    assert_eq!(sv.insert_into_hole(1, 'd'), Err('d'));
    assert_eq!(sv.insert_into_hole(2, 'c'), Err('c'));
    assert_eq!(sv.num_elements(), 2);
    assert_eq!(sv.clone().into_vec(), &['a', 'c']);

    sv.grow(3);
    assert_eq!(sv.num_elements(), 2);
    assert_eq!(sv.clone().into_vec(), &['a', 'c']);

    assert_eq!(sv.insert_into_hole(4, 'd'), Ok(()));
    assert_eq!(sv.insert_into_hole(4, 'e'), Err('e'));

    assert_eq!(sv.num_elements(), 3);
    assert_eq!(sv.clone().into_vec(), &['a', 'c', 'd']);
}

#[test]
fn size_hints() {
    let mut sv = StableVec::<()>::new();

    assert_eq!(sv.iter().size_hint(), (0, Some(0)));
    assert_eq!(sv.iter_mut().size_hint(), (0, Some(0)));
    assert_eq!(sv.keys().size_hint(), (0, Some(0)));


    let mut sv = StableVec::from(&[0, 1, 2, 3, 4]);
    sv.remove(1);

    macro_rules! check_iter {
        ($it:expr) => {{
            let mut it = $it;
            assert_eq!(it.size_hint(), (4, Some(4)));
            assert!(it.next().is_some());
            assert_eq!(it.size_hint(), (3, Some(3)));
            assert!(it.next().is_some());
            assert_eq!(it.size_hint(), (2, Some(2)));
            assert!(it.next().is_some());
            assert_eq!(it.size_hint(), (1, Some(1)));
            assert!(it.next().is_some());
            assert_eq!(it.size_hint(), (0, Some(0)));
        }}
    }

    check_iter!(sv.iter());
    check_iter!(sv.iter_mut());
    check_iter!(sv.keys());
}
