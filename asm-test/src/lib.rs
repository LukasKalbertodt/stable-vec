//! This is just a dummy library to check the assembly output of some
//! functions.

use stable_vec::{StableVec, OptionCore, BitVecCore};

type SvOption<T> = StableVec<T, OptionCore<T>>;
type SvBitVec<T> = StableVec<T, BitVecCore<T>>;

pub fn index_u32_option(sv: &SvOption<u32>, index: usize) -> u32 {
    sv[index]
}

pub fn push_u32_option(sv: &mut SvOption<u32>, val: u32) -> usize {
    sv.push(val)
}

pub fn count_u32_option(sv: &SvOption<u32>) -> usize {
    sv.indices().count()
}

pub fn index_u32_bitvec(sv: &SvBitVec<u32>, index: usize) -> u32 {
    sv[index]
}

pub fn push_u32_bitvec(sv: &mut SvBitVec<u32>, val: u32) -> usize {
    sv.push(val)
}

pub fn count_u32_bitvec(sv: &SvBitVec<u32>) -> usize {
    sv.indices().count()
}
