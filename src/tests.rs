use std::{
    fmt::Debug,
    panic::RefUnwindSafe,
};
use crate::{Core, StableVecFacade};

macro_rules! assert_panic {
    ($($body:tt)*) => {{
        let res = std::panic::catch_unwind(|| {
            $($body)*
        });
        if let Ok(x) = res {
            panic!(
                "expected panic for '{}', but got '{:?}' ",
                stringify!($($body)*),
                x,
            );
        }
    }}
}

fn assert_sv_eq_fn<T, C>(
    sv: &mut StableVecFacade<T, C>,
    indices: &[usize],
    values: &mut [T],
    last_index: usize,
)
where
    T: Debug + PartialEq + Copy + RefUnwindSafe,
    C: Core<T> + RefUnwindSafe + Clone,
{
    let num_elements = values.len();

    assert_eq!(sv.num_elements(), num_elements, "num_elements check failed");
    assert_eq!(sv.is_empty(), num_elements == 0, "is_empty check failed");
    assert_eq!(sv.is_compact(), last_index + 1 == num_elements, "is_compact check failed");
    assert_eq!(sv.next_index(), last_index + 1, "next_index check failed");
    assert!(sv.capacity() >= last_index + 1, "capacity check failed");

    assert_eq!(sv.iter().cloned().collect::<Vec<_>>(), values);
    assert_eq!(sv.iter_mut().map(|r| *r).collect::<Vec<_>>(), values);
    assert_eq!((&*sv).into_iter().cloned().collect::<Vec<_>>(), values);
    assert_eq!((&mut *sv).into_iter().map(|r| *r).collect::<Vec<_>>(), values);
    assert_eq!((*sv).clone().into_iter().collect::<Vec<_>>(), values);
    assert_eq!(sv.indices().collect::<Vec<_>>(), indices);

    let expected_hint = (num_elements, Some(num_elements));
    assert_eq!(sv.iter().cloned().len(), num_elements);
    assert_eq!(sv.iter().cloned().size_hint(), expected_hint);
    assert_eq!(sv.iter_mut().map(|r| *r).size_hint(), expected_hint);
    assert_eq!(sv.iter_mut().map(|r| *r).len(), num_elements);
    assert_eq!((&*sv).into_iter().cloned().size_hint(), expected_hint);
    assert_eq!((&mut *sv).into_iter().map(|r| *r).size_hint(), expected_hint);
    assert_eq!(sv.into_iter().size_hint(), expected_hint);
    assert_eq!(sv.into_iter().len(), num_elements);
    assert_eq!(sv.indices().size_hint(), expected_hint);
    assert_eq!(sv.indices().len(), num_elements);

    assert_eq!(sv, &*values);
    assert_eq!(sv, &values.to_vec());

    assert_eq!(format!("{:?}", sv), format!("StableVec {:?}", values));
    assert_eq!(sv.clone().into_vec(), values);

    for i in 0..last_index {
        if let Ok(index_index) = indices.binary_search(&i) {
            assert!(sv.has_element_at(i));
            assert_eq!(sv.get(i), Some(&values[index_index]));
            assert_eq!(sv.get_mut(i), Some(&mut values[index_index]));
            assert_eq!(sv[i], values[index_index]);
        } else {
            assert!(!sv.has_element_at(i));
            assert_eq!(sv.get(i), None);
            assert_eq!(sv.get_mut(i), None);
            assert_panic!(sv[i]);
        }
    }
}

macro_rules! assert_sv_eq {
    ($left:expr, [$(; $last_index:literal)*]: $ty:ty $(,)*) => {{
        let sv = &mut $left;

        let last_index = 0 $(+ $last_index)*;
        let next_index = if last_index == 0 { 0 } else { last_index + 1 };

        assert_eq!(sv.num_elements(), 0, "num_elements check failed");
        assert!(sv.is_empty(), "is_empty() check failed");
        assert_eq!(sv.is_compact(), next_index == 0, "is_compact check failed");
        assert_eq!(sv.next_index(), next_index, "next_index check failed");
        assert!(sv.capacity() >= next_index, "capacity check failed");

        assert_eq!(sv.iter().count(), 0);
        assert_eq!(sv.iter_mut().count(), 0);
        assert_eq!((&*sv).into_iter().count(), 0);
        assert_eq!((&mut *sv).into_iter().count(), 0);
        assert_eq!(sv.into_iter().count(), 0);
        assert_eq!(sv.indices().count(), 0);

        assert_eq!(sv, &[] as &[$ty]);
        assert_eq!(sv, &vec![] as &Vec<$ty>);

        assert_eq!(format!("{:?}", sv), "StableVec []");
        assert!(sv.clone().into_vec().is_empty());
    }};
    ($left:expr, [$( $idx:literal => $val:expr ),* $(; $last_index:literal)*] $(,)*) => {{
        let indices = [$($idx),*];
        let mut values = [$($val),*];
        let last_index = 0 $(+ $last_index)*;
        let last_index = if last_index == 0 {
            *indices.last().unwrap()
        } else {
            last_index
        };

        assert_sv_eq_fn(&mut $left, &indices, &mut values, last_index);
    }};
}

macro_rules! gen_tests_for {
    ($ty:ident) => {
        #[test]
        fn new() {
            let mut sv = $ty::<String>::new();
            assert_sv_eq!(sv, []: String);
        }

        #[test]
        fn default() {
            let mut sv: $ty<String> = $ty::default();
            assert_sv_eq!(sv, []: String);
        }

        #[test]
        fn with_capacity() {
            let mut sv: $ty<String> = $ty::with_capacity(3);

            assert!(sv.capacity() >= 3);
            assert_sv_eq!(sv, []: String);
        }

        #[test]
        fn reserve() {
            let mut sv = $ty::<String>::new();

            // Reserve for 5
            sv.reserve(5);
            assert!(sv.capacity() >= 5);
            assert_sv_eq!(sv, []: String);

            // Reserve for 2 more
            sv.reserve(7);
            assert!(sv.capacity() >= 7);
            assert_sv_eq!(sv, []: String);

            // Reserving for 6 should do nothing because we already have memory for 7
            // or more!
            let cap_before = sv.capacity();
            sv.reserve(6);
            assert_eq!(sv.capacity(), cap_before);
            assert_sv_eq!(sv, []: String);

            // After pushing 23 elements, we should have at least memory for 23 items.
            for _ in 0..23 {
                sv.push("x".into());
            }
            assert!(sv.capacity() >= 23);

            // Reserving for 13 more elements
            sv.reserve(13);
            assert!(sv.capacity() >= 36);

            // Reserving for 2 more shouldn't do anything because we already reserved
            // for 13 additional ones.
            let cap_before = sv.capacity();
            sv.reserve(2);
            assert_eq!(sv.capacity(), cap_before);
        }

        #[test]
        fn from_vec() {
            assert_sv_eq!(
                $ty::<String>::from_vec(vec![]),
                []: String,
            );

            assert_sv_eq!(
                $ty::from_vec(vec![1]),
                [0 => 1],
            );

            assert_sv_eq!(
                $ty::from_vec(vec![2, 9, 5]),
                [0 => 2, 1 => 9, 2 => 5],
            );
        }

        #[test]
        fn from() {
            assert_sv_eq!(
                $ty::<String>::from(&[]),
                []: String,
            );

            assert_sv_eq!(
                $ty::from(&[1]),
                [0 => 1],
            );

            assert_sv_eq!(
                $ty::from(&[2, 9, 5]),
                [0 => 2, 1 => 9, 2 => 5],
            );
        }

        #[test]
        fn push_simple() {
            let mut sv = $ty::new();

            sv.push('a');
            assert_sv_eq!(sv, [0 => 'a']);

            sv.push('b');
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b']);

            sv.push('c');
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c']);
        }

        #[test]
        fn remove_first() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c']);

            assert_eq!(sv.remove_first(), Some('a'));
            assert_sv_eq!(sv, [1 => 'b', 2 => 'c'; 2]);

            assert_eq!(sv.remove_first(), Some('b'));
            assert_sv_eq!(sv, [2 => 'c'; 2]);

            sv.push('d');
            assert_sv_eq!(sv, [2 => 'c', 3 => 'd']);

            sv.push('e');
            assert_sv_eq!(sv, [2 => 'c', 3 => 'd', 4 => 'e']);

            assert_eq!(sv.remove_first(), Some('c'));
            assert_sv_eq!(sv, [3 => 'd', 4 => 'e'; 4]);

            assert_eq!(sv.remove_first(), Some('d'));
            assert_sv_eq!(sv, [4 => 'e'; 4]);

            assert_eq!(sv.remove_first(), Some('e'));
            assert_sv_eq!(sv, [; 4]: char);
        }

        #[test]
        fn remove_last() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c']);

            assert_eq!(sv.remove_last(), Some('c'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 2]);

            assert_eq!(sv.remove_last(), Some('b'));
            assert_sv_eq!(sv, [0 => 'a'; 2]);

            sv.push('d');
            assert_sv_eq!(sv, [0 => 'a', 3 => 'd']);

            sv.push('e');
            assert_sv_eq!(sv, [0 => 'a', 3 => 'd', 4 => 'e']);

            assert_eq!(sv.remove_last(), Some('e'));
            assert_sv_eq!(sv, [0 => 'a', 3 => 'd'; 4]);

            assert_eq!(sv.remove_last(), Some('d'));
            assert_sv_eq!(sv, [0 => 'a'; 4]);

            assert_eq!(sv.remove_last(), Some('a'));
            assert_sv_eq!(sv, [; 4]: char);
        }

        #[test]
        fn find_first() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            assert_eq!(sv.find_first(), Some(&'a'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 1]);

            sv.push('c');
            assert_eq!(sv.find_first(), Some(&'a'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c'; 2]);

            sv.remove(1);
            assert_eq!(sv.find_first(), Some(&'a'));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c'; 2]);

            sv.remove(0);
            assert_eq!(sv.find_first(), Some(&'c'));
            assert_sv_eq!(sv, [2 => 'c'; 2]);

            sv.clear();
            assert_eq!(sv.find_first(), None);
            assert_sv_eq!(sv, []: char);
        }

        #[test]
        fn find_first_mut() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            *sv.find_first_mut().unwrap() = 'c';
            assert_sv_eq!(sv, [0 => 'c', 1 => 'b'; 1]);

            sv.remove(0);
            *sv.find_first_mut().unwrap() = 'd';
            assert_sv_eq!(sv, [1 => 'd'; 1]);

            sv.remove(1);
            assert_eq!(sv.find_first_mut(), None);
            assert_sv_eq!(sv, [; 1]: char);
        }

        #[test]
        fn find_last() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            assert_eq!(sv.find_last(), Some(&'b'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 1]);

            sv.push('c');
            assert_eq!(sv.find_last(), Some(&'c'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c'; 2]);

            sv.remove(1);
            assert_eq!(sv.find_last(), Some(&'c'));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c'; 2]);

            sv.remove(2);
            assert_eq!(sv.find_last(), Some(&'a'));
            assert_sv_eq!(sv, [0 => 'a'; 2]);

            sv.clear();
            assert_eq!(sv.find_last(), None);
            assert_sv_eq!(sv, []: char);
        }

        #[test]
        fn find_last_mut() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            *sv.find_last_mut().unwrap() = 'c';
            assert_sv_eq!(sv, [0 => 'a', 1 => 'c'; 1]);

            sv.remove(1);
            *sv.find_last_mut().unwrap() = 'd';
            assert_sv_eq!(sv, [0 => 'd'; 1]);

            sv.remove(0);
            assert_eq!(sv.find_last_mut(), None);
            assert_sv_eq!(sv, [; 1]: char);
        }

        #[test]
        fn find_first_index() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            assert_eq!(sv.find_first_index(), Some(0));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 1]);

            sv.remove(0);
            assert_eq!(sv.find_first_index(), Some(1));
            assert_sv_eq!(sv, [1 => 'b'; 1]);

            sv.push('c');
            assert_eq!(sv.find_first_index(), Some(1));
            assert_sv_eq!(sv, [1 => 'b', 2 => 'c'; 2]);

            sv.remove(1);
            assert_eq!(sv.find_first_index(), Some(2));
            assert_sv_eq!(sv, [2 => 'c'; 2]);

            sv.remove(2);
            assert_eq!(sv.find_first_index(), None);
            assert_sv_eq!(sv, [; 2]: char);
        }

        #[test]
        fn find_last_index() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            assert_eq!(sv.find_last_index(), Some(1));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 1]);

            sv.remove(1);
            assert_eq!(sv.find_last_index(), Some(0));
            assert_sv_eq!(sv, [0 => 'a'; 1]);

            sv.push('c');
            assert_eq!(sv.find_last_index(), Some(2));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c'; 2]);

            sv.remove(2);
            assert_eq!(sv.find_last_index(), Some(0));
            assert_sv_eq!(sv, [0 => 'a'; 2]);

            sv.remove(0);
            assert_eq!(sv.find_last_index(), None);
            assert_sv_eq!(sv, [; 2]: char);
        }

        #[test]
        fn retain_indices() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c', 'd', 'e']);

            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c', 3 => 'd', 4 => 'e'; 4]);

            sv.retain_indices(|index| index != 2);
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 3 => 'd', 4 => 'e'; 4]);

            sv.retain_indices(|index| index == 0 || index == 3);
            assert_sv_eq!(sv, [0 => 'a', 3 => 'd'; 4]);

            sv.retain_indices(|index| index == 0);
            assert_sv_eq!(sv, [0 => 'a'; 4]);

            sv.retain_indices(|index| index != 4);
            assert_sv_eq!(sv, [0 => 'a'; 4]);

            sv.retain_indices(|_| false);
            assert_sv_eq!(sv, [; 4]: char);
        }

        #[test]
        fn grow() {
            let mut sv = $ty::from_vec(vec!['a', 'b']);

            sv.grow(1);
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 2]);

            sv.grow(9);
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b'; 11]);
        }

        #[test]
        fn shrink_to_fit() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c', 'd', 'e', 'f']);
            sv.reserve(100);
            sv.retain_indices(|index| index != 1 && index != 3 && index != 5);
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 4 => 'e'; 5]);

            sv.shrink_to_fit();
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 4 => 'e'; 5]);
            assert_eq!(sv.capacity(), 6);
        }

        #[test]
        fn remove() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c']);

            assert_eq!(sv.remove(1), Some('b'));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c']);

            assert_eq!(sv.remove(3), None);
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c']);

            sv.extend_from_slice(&['d', 'e']);
            assert_eq!(sv.remove(4), Some('e'));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 3 => 'd'; 4]);

            assert_eq!(sv.remove(4), None);
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 3 => 'd'; 4]);

            assert_eq!(sv.remove(5), None);
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 3 => 'd'; 4]);

            assert_eq!(sv.remove(1), None);
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c', 3 => 'd'; 4]);

            assert_eq!(sv.remove(0), Some('a'));
            assert_sv_eq!(sv, [2 => 'c', 3 => 'd'; 4]);
        }

        #[test]
        fn large() {
            let mut sv = $ty::new();

            const LIMIT: usize = 200;
            sv.extend((0u32..LIMIT as u32).rev());
            sv.push(2 * LIMIT as u32);

            for i in 0..LIMIT {
                assert!(sv.has_element_at(i));
                assert_eq!(sv.get(i).copied(), Some(LIMIT as u32 - 1 - i as u32));
            }
            assert_eq!(sv.num_elements(), LIMIT + 1);
            assert_eq!(sv.indices().count(), LIMIT + 1);
            assert_eq!(sv.iter().count(), LIMIT + 1);
            assert_eq!(sv.iter_mut().count(), LIMIT + 1);
            assert_eq!(sv.clone().into_iter().count(), LIMIT + 1);

            for hole in 0..LIMIT {
                let mut clone = sv.clone();
                clone.remove(hole);

                for i in 0..LIMIT {
                    if i != hole {
                        assert!(clone.has_element_at(i));
                        assert_eq!(clone.get(i).copied(), Some(LIMIT as u32 - 1 - i as u32));
                    } else {
                        assert!(!clone.has_element_at(i));
                        assert_eq!(clone.get(i), None);
                    }
                }
            }
        }

        #[test]
        fn zero_sized_type() {
            println!("hi");
            println!("hi");
            println!("hi");
            println!("hi");
            let mut sv = $ty::<()>::from(&[(), (), ()]);

            assert_eq!(sv.remove(1), Some(()));
            assert_sv_eq!(sv, [0 => (), 2 => ()]);

            assert_eq!(sv.remove(3), None);
            assert_sv_eq!(sv, [0 => (), 2 => ()]);

            sv.extend_from_slice(&[(), ()]);
            assert_eq!(sv.remove(4), Some(()));
            assert_sv_eq!(sv, [0 => (), 2 => (), 3 => (); 4]);

            assert_eq!(sv.remove(4), None);
            assert_sv_eq!(sv, [0 => (), 2 => (), 3 => (); 4]);

            assert_eq!(sv.remove(5), None);
            assert_sv_eq!(sv, [0 => (), 2 => (), 3 => (); 4]);

            assert_eq!(sv.remove(1), None);
            assert_sv_eq!(sv, [0 => (), 2 => (), 3 => (); 4]);

            assert_eq!(sv.remove(0), Some(()));
            assert_sv_eq!(sv, [2 => (), 3 => (); 4]);
        }

        #[test]
        fn insert_into_hole() {
            let mut sv = $ty::from_vec(vec!['a', 'b', 'c']);

            assert_eq!(sv.insert_into_hole(1, 'x'), Err('x'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c']);

            assert_eq!(sv.insert_into_hole(3, 'x'), Err('x'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c']);

            assert_eq!(sv.remove(1), Some('b'));
            assert_eq!(sv.insert_into_hole(1, 'd'), Ok(()));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'd', 2 => 'c']);

            assert_eq!(sv.insert_into_hole(1, 'x'), Err('x'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'd', 2 => 'c']);

            assert_eq!(sv.remove(1), Some('d'));
            assert_sv_eq!(sv, [0 => 'a', 2 => 'c']);

            assert_eq!(sv.insert_into_hole(1, 'e'), Ok(()));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'e', 2 => 'c']);

            sv.grow(2);
            assert_eq!(sv.insert_into_hole(5, 'x'), Err('x'));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'e', 2 => 'c'; 4]);

            assert_eq!(sv.insert_into_hole(4, 'f'), Ok(()));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'e', 2 => 'c', 4 => 'f']);

            assert_eq!(sv.insert_into_hole(3, 'g'), Ok(()));
            assert_sv_eq!(sv, [0 => 'a', 1 => 'e', 2 => 'c', 3 => 'g', 4 => 'f']);
        }

        #[test]
        fn clear() {
            let mut sv: $ty<String> = $ty::new();
            sv.clear();
            assert_sv_eq!(sv, []: String);

            let mut sv = $ty::from_vec(vec![1, 3, 5]);
            sv.clear();
            assert_sv_eq!(sv, []: u32);
        }

        #[test]
        fn extend_from_slice() {
            let mut sv = $ty::new();

            sv.extend_from_slice(&['a']);
            assert_sv_eq!(sv, [0 => 'a']);

            sv.push('b');
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b']);

            sv.extend_from_slice(&['c', 'd']);
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c', 3 => 'd']);

            assert_eq!(sv.remove_last(), Some('d'));
            sv.extend_from_slice(&['e']);
            assert_sv_eq!(sv, [0 => 'a', 1 => 'b', 2 => 'c', 4 => 'e']);
        }

        #[test]
        fn write() {
            use std::io::Write;

            let mut sv = $ty::new();

            sv.write_all(&[0, 7, 3]).unwrap();
            assert_sv_eq!(sv, [0 => 0, 1 => 7, 2 => 3]);

            sv.remove_last();
            sv.write_all(&[4, 8]).unwrap();
            assert_sv_eq!(sv, [0 => 0, 1 => 7, 3 => 4, 4 => 8]);

            sv.write_all(&[5]).unwrap();
            assert_sv_eq!(sv, [0 => 0, 1 => 7, 3 => 4, 4 => 8, 5 => 5]);
        }

        #[test]
        fn clone() {
            let sv = $ty::<String>::new();
            assert_sv_eq!(sv.clone(), []: String);

            let sv = $ty::from(&[2, 4]);
            assert_sv_eq!(sv.clone(), [0 => 2, 1 => 4]);

            let mut sv = $ty::from(&[2, 5, 4]);
            sv.remove(1);
            assert_sv_eq!(sv.clone(), [0 => 2, 2 => 4]);
        }

        #[test]
        fn iter_mut() {
            let mut sv = $ty::from(&[2, 5, 4]);

            for x in &mut sv {
                *x *= 2;
            }
            assert_sv_eq!(sv, [0 => 4, 1 => 10, 2 => 8]);

            for x in sv.iter_mut() {
                *x -= 1;
            }
            assert_sv_eq!(sv, [0 => 3, 1 => 9, 2 => 7]);
        }

        #[test]
        fn index_mut() {
            let mut sv = $ty::from(&[2, 5, 4]);

            sv[1] = 8;
            assert_sv_eq!(sv, [0 => 2, 1 => 8, 2 => 4]);

            sv[2] = 5;
            assert_sv_eq!(sv, [0 => 2, 1 => 8, 2 => 5]);
        }

        #[test]
        fn index_panic() {
            let mut sv: $ty<_> = $ty::from(&[2, 5, 4]);
            sv.remove(1);

            assert_panic!(sv[1]);
            assert_panic!(sv[3]);

            sv.reserve(10);
            assert_panic!(sv[8]);
        }

        #[test]
        fn correct_drop() {
            use std::sync::atomic::{Ordering, AtomicIsize};

            static ALIVE_COUNT: AtomicIsize = AtomicIsize::new(0);

            struct Dummy(char);
            impl Dummy {
                fn new(c: char) -> Self {
                    ALIVE_COUNT.fetch_add(1, Ordering::SeqCst);
                    Self(c)
                }
            }
            impl Drop for Dummy {
                fn drop(&mut self) {
                    ALIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
                }
            }
            impl Clone for Dummy {
                fn clone(&self) -> Self {
                    Self::new(self.0)
                }
            }

            let mut sv = $ty::new();

            sv.push(Dummy::new('a'));
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);

            sv.push(Dummy::new('b'));
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

            sv.push(Dummy::new('c'));
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 3);

            sv.remove(1);
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

            sv.extend_from_slice(&[Dummy::new('d'), Dummy::new('e')]);
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 4);

            sv.remove_first();
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 3);

            sv.retain(|c| c.0 != 'd');
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

            {
                let mut clone = sv.clone();
                assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 4);

                clone.reordering_make_compact();
                assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 4);
            }
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);


            sv.make_compact();
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

            sv.clear();
            assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);
        }

        #[test]
        fn compact_tiny() {
            let mut sv = $ty::from(&[1.0, 2.0, 3.0]);
            sv.remove(1);
            assert_sv_eq!(sv, [0 => 1.0, 2 => 3.0]);

            sv.make_compact();
            assert_sv_eq!(sv, [0 => 1.0, 1 => 3.0]);
            assert_eq!(sv.into_vec(), &[1.0, 3.0]);
        }

        #[test]
        fn insert_into_hole_and_grow() {
            let mut sv = $ty::from(&['a', 'b']);
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
        fn extend_from_iter() {
            use std::iter::FromIterator;

            let sv = $ty::from_iter(0..0);
            assert_sv_eq!(sv.clone(), []: u32);

            let sv = $ty::from_iter(0..3);
            assert_sv_eq!(sv.clone(), [0 => 0, 1 => 1, 2 => 2]);

            let mut sv = $ty::from_iter((0..3).map(|x| x * 3));
            assert_sv_eq!(sv.clone(), [0 => 0, 1 => 3, 2 => 6]);

            sv.remove(2);
            sv.extend((7..10).rev());
            assert_sv_eq!(sv.clone(), [0 => 0, 1 => 3, 3 => 9, 4 => 8, 5 => 7]);
        }

        #[test]
        fn size_hints() {
            let mut sv = $ty::<()>::new();

            assert_eq!(sv.iter().size_hint(), (0, Some(0)));
            assert_eq!(sv.iter_mut().size_hint(), (0, Some(0)));
            assert_eq!(sv.indices().size_hint(), (0, Some(0)));


            let mut sv = $ty::from(&[0, 1, 2, 3, 4]);
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
            check_iter!(sv.indices());
        }

        quickcheck! {
            fn reordering_compact(insertions: u16, to_delete: Vec<u16>) -> bool {
                let insertions = insertions + 1;
                // Create stable vector containing `insertions` zeros. Afterwards, we
                // remove at most half of those elements
                let mut sv = $ty::from(vec![0; insertions as usize]);
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
                let mut sv = $ty::from(vec![0; insertions as usize]);
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

                let sv_a = $ty::from_iter(iter_a);
                let sv_b = {
                    let mut sv = $ty::new();
                    sv.extend(iter_b);
                    sv
                };
                let sv_c = $ty::from(&items);

                sv_a.num_elements() == items.len()
                    && sv_a == sv_b
                    && sv_a == sv_c
            }
        }
    }
}

mod option {
    use quickcheck::quickcheck;
    use crate::InlineStableVec;
    use super::assert_sv_eq_fn;

    gen_tests_for!(InlineStableVec);
}

mod bitvec {
    use quickcheck::quickcheck;
    use crate::ExternStableVec;
    use super::assert_sv_eq_fn;

    gen_tests_for!(ExternStableVec);
}
