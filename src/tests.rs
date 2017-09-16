use super::StableVec;

quickcheck! {
    fn compact(insertions: u16, to_delete: Vec<u16>) -> bool {
        let insertions = insertions + 1;
        // Create stable vector containing `insertions` zeros. Afterwards, we
        // remove at most half of those elements
        let mut sv = StableVec::from(vec![0; insertions as usize]);
        for i in to_delete {
            let i = (i % insertions) as usize;
            if sv.exists(i) {
                sv.remove(i);
            }
        }

        // Remember the number of elements before and call compact.
        let n_before_compact = sv.num_elements();
        sv.compact();

        n_before_compact == sv.num_elements()
            && sv.is_compact()
            && (0..n_before_compact).all(|i| sv.get(i).is_some())
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
