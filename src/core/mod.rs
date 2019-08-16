//! `Core` trait definition and implementations.
//!
//! There are multiple ways to implement the "stable vector" interface, each
//! with different performance characteristics. The `Core` is this
//! implementation, making the stable vector work. See [`Core`][core::Core] for
//! more information.

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub use self::option::OptionCore;
pub use self::bitvec::BitVecCore;

mod option;
mod bitvec;


/// The default core implementation of the stable vector. Fine in most
/// situations.
pub type DefaultCore<T> = BitVecCore<T>;

/// The core of a stable vector.
///
/// *Note*: If you are a user of this crate, you probably don't care about
/// this! See the documentation on [`StableVecFacade`][crate::StableVecFacade]
/// and the different core implementations for more useful information. This
/// trait is only important for you if you want to implement your own core
/// implementation.
///
/// Implementors of the trait take the core role in the stable vector: storing
/// elements of type `T` where each element might be deleted. The elements can
/// be referred to by an index.
///
/// Core types must never read deleted elements in `drop()`. So they must
/// ensure to only ever drop existing elements.
///
///
/// # Formal semantics
///
/// A core defines a map from `usize` (the so called "indices") to elements of
/// type `Option<T>`. It has a length (`len`) and a capacity (`cap`).
///
/// It's best to think of this as a contiguous sequence of "slots". A slot can
/// either be empty or filled with an element. A core has always `cap` many
/// slots. Here is an example of such a core with `len = 8` and `cap = 10`.
///
/// ```text
///      0   1   2   3   4   5   6   7   8   9   10
///    ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///    │ a │ - │ b │ c │ - │ - │ d │ - │ - │ - │
///    └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                                      ↑       ↑
///                                     len     cap
/// ```
///
/// `len` and `cap` divide the index space into three parts, which have the
/// following invariants:
/// - `0 ≤ i < len`: slots with index `i` can be empty or filled
/// - `len ≤ i < cap`: slots with index `i` are always empty
/// - `cap ≤ i`: slots with index `i` are undefined (all methods dealing with
///   indices will exhibit undefined behavior when the index is `≥ cap`)
///
/// Additional required invariants:
/// - `len ≤ cap`
/// - `cap ≤ isize::MAX`
/// - Methods with `&self` receiver do not change anything observable about the
///   core.
///
/// These invariants must not (at any time) be violated by users of this API.
///
/// Cloning a core must clone everything, including all empty slots. This means
/// that the capacity of the clone must be at least the capacity of the
/// original value.
pub trait Core<T> {
    /// Creates an empty instance without any elements. Must not allocate
    /// memory.
    ///
    /// # Formal
    ///
    /// **Postconditons** (of returned instance `out`):
    /// - `out.len() == 0`
    /// - `out.cap() == 0`
    fn new() -> Self;

    /// Returns the length of this core (the `len`). See the trait docs for
    /// more information.
    fn len(&self) -> usize;

    /// Sets the `len` to a new value.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `new_len ≤ self.cap()`
    /// - ∀ i in `new_len..self.cap()` ⇒ `self.has_element_at(i) == false`
    ///
    /// **Invariants**:
    /// - *slot data*
    ///
    /// **Postconditons**:
    /// - `self.len() == new_len`
    unsafe fn set_len(&mut self, new_len: usize);

    /// Returns the capacity of this core (the `cap`). See the trait docs for
    /// more information.
    fn cap(&self) -> usize;

    /// Reallocates the memory to have a `cap` of at least `new_cap`. This
    /// method should try its best to allocate exactly `new_cap`.
    ///
    /// This means that after calling this method, inserting elements at
    /// indices in the range `0..new_cap` is valid. This method shall not check
    /// if there is already enough capacity available.
    ///
    /// For implementors: please mark this impl with `#[cold]` and
    /// `#[inline(never)]`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `new_cap ≥ self.len()` (as a consequence, this method does not (need
    ///   to) drop elements; all slots >= `new_cap` are empty)
    /// - `new_cap ≤ isize::MAX`
    ///
    /// **Invariants**:
    /// - *slot data*
    /// - `self.len()`
    ///
    /// **Postconditons**:
    /// - `self.cap() >= new_cap`
    unsafe fn realloc(&mut self, new_cap: usize);

    /// Checks if there exists an element with index `idx`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()`
    unsafe fn has_element_at(&self, idx: usize) -> bool;

    /// Inserts `elem` at the index `idx`. Does *not* updated the `used_len`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()`
    /// - `self.has_element_at(idx) == false`
    ///
    /// **Invariants**:
    /// - `self.len()`
    /// - `self.cap()`
    ///
    /// **Postconditons**:
    /// - `self.get_unchecked(idx) == elem`
    unsafe fn insert_at(&mut self, idx: usize, elem: T);

    /// Removes the element at index `idx` and returns it.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()`
    /// - `self.has_element_at(idx) == true`
    ///
    /// **Invariants**:
    /// - `self.len()`
    /// - `self.cap()`
    ///
    /// **Postconditons**:
    /// - `self.has_element_at(idx) == false`
    unsafe fn remove_at(&mut self, idx: usize) -> T;

    /// Returns a reference to the element at the index `idx`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()`
    /// - `self.has_element_at(idx) == true` (implying `idx < self.len()`)
    unsafe fn get_unchecked(&self, idx: usize) -> &T;

    /// Returns a mutable reference to the element at the index `idx`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()`
    /// - `self.has_element_at(idx) == true` (implying `idx < self.len()`)
    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T;

    /// Deletes all elements without deallocating memory. Drops all existing
    /// elements. Sets `len` to 0.
    ///
    /// # Formal
    ///
    /// **Invariants**:
    /// - `self.cap()`
    ///
    /// **Postconditons**:
    /// - `self.len() == 0` (implying all slots are empty)
    fn clear(&mut self);

    /// Returns the index of the next filled slot with index `idx` or higher.
    /// Specifically, if an element at index `idx` exists, `Some(idx)` is
    /// returned.
    ///
    /// The inputs `idx >= self.len()` are only allowed for convenience and
    /// because it doesn't make the implementation more complicated. For those
    /// `idx` values, `None` is always returned.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx ≤ self.cap()`
    ///
    /// **Postconditons** (for return value `out`):
    /// - if `out == None`:
    ///     - ∀ i in `idx..self.len()` ⇒ `self.has_element_at(i) == false`
    /// - if `out == Some(j)`:
    ///     - ∀ i in `idx..j` ⇒ `self.has_element_at(i) == false`
    ///     - `self.has_element_at(j) == true`
    unsafe fn next_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.cap());

        (idx..self.len()).find(|&idx| self.has_element_at(idx))
    }

    /// Returns the index of the previous filled slot with index `idx` or
    /// lower. Specifically, if an element at index `idx` exists, `Some(idx)`
    /// is returned.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx < self.cap()` (Note: unlike `next_index_from`, this doesn't
    ///   allow equality here. Also: passing in `idx >= self.len()` just wastes
    ///   time, as those slots are never filled.)
    ///
    /// **Postconditons** (for return value `out`):
    /// - if `out == None`:
    ///     - ∀ i in `0..=idx` ⇒ `self.has_element_at(i) == false`
    /// - if `out == Some(j)`:
    ///     - ∀ i in `j + 1..=idx` ⇒ `self.has_element_at(i) == false`
    ///     - `self.has_element_at(j) == true`
    unsafe fn prev_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx < self.cap());

        (0..=idx).rev().find(|&idx| self.has_element_at(idx))
    }

    /// Returns the index of the next empty slot with index i where `idx ≤ i <
    /// self.cap()`.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `idx ≤ self.cap()`
    ///
    /// **Postconditons** (for return value `out`):
    /// - if `out == None`:
    ///     - ∀ i in `idx..self.len()` ⇒ `self.has_element_at(i) == true`
    /// - if `out == Some(j)`:
    ///     - ∀ i in `idx..j` ⇒ `self.has_element_at(i) == true`
    ///     - `self.has_element_at(j) == false`
    unsafe fn next_hole_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.cap());

        (idx..self.cap()).find(|&idx| !self.has_element_at(idx))
    }

    /// Swaps the two slots with indices `a` and `b`. That is: the element
    /// *and* the "filled/empty" status are swapped. The slots at indices `a`
    /// and `b` can be empty or filled.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `a < self.cap()`
    /// - `b < self.cap()`
    ///
    /// **Invariants**:
    /// - `self.len()`
    /// - `self.cap()`
    ///
    /// **Postconditons** (with `before` being `self` before the call):
    /// - `before.has_element_at(a) == self.has_element_at(b)`
    /// - `before.has_element_at(b) == self.has_element_at(a)`
    /// - if `self.has_element_at(a)`:
    ///     - `self.get_unchecked(a) == before.get_unchecked(b)`
    /// - if `self.has_element_at(b)`:
    ///     - `self.get_unchecked(b) == before.get_unchecked(a)`
    unsafe fn swap(&mut self, a: usize, b: usize);
}


/// Just a wrapper around a core with a `PhantomData<T>` field to signal
/// ownership of `T` (for variance and for the drop checker).
///
/// Implements `Deref` and `DerefMut`, returning the actual core. This is just
/// a helper so that not all structs storing a core have to also have a
/// `PhantomData` field.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub(crate) struct OwningCore<T, C: Core<T>> {
    core: C,
    _dummy: PhantomData<T>,
}

impl<T, C: Core<T>> OwningCore<T, C> {
    pub(crate) fn new(core: C) -> Self {
        Self {
            core,
            _dummy: PhantomData,
        }
    }
}


impl<T, C: Core<T>> Deref for OwningCore<T, C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl<T, C: Core<T>> DerefMut for OwningCore<T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}
