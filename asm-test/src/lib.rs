//! This is just a dummy library to check the assembly output of some
//! functions.

use stable_vec::StableVec;

pub fn index_u32_option(sv: &StableVec<u32>, index: usize) -> u32 {
    sv[index]
}

pub fn push_u32_option(sv: &mut StableVec<u32>, val: u32) -> usize {
    sv.push(val)
}
