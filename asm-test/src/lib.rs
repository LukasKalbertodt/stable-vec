//! This is just a dummy library to check the assembly output of some
//! functions.

use stable_vec::{InlineStableVec, ExternStableVec};


pub fn index_u32_option(sv: &InlineStableVec<u32>, index: usize) -> u32 {
    sv[index]
}

pub fn push_u32_option(sv: &mut InlineStableVec<u32>, val: u32) -> usize {
    sv.push(val)
}

pub fn count_u32_option(sv: &InlineStableVec<u32>) -> usize {
    sv.indices().count()
}

pub fn index_u32_bitvec(sv: &ExternStableVec<u32>, index: usize) -> u32 {
    sv[index]
}

pub fn push_u32_bitvec(sv: &mut ExternStableVec<u32>, val: u32) -> usize {
    sv.push(val)
}

pub fn count_u32_bitvec(sv: &ExternStableVec<u32>) -> usize {
    sv.indices().count()
}
