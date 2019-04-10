use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub(crate) mod option;


/// The core of a stable vector: conceptually a `Vec<Option<T>>`.
///
/// Implementors of the trait take the core role in the stable vector: storing
/// elements of type `T` where each element might be deleted. The elements can
/// be referred to by an index.
///
/// Regarding drop behavior: the core types must never read deleted elements in
/// `drop()`. So they must ensure to only ever drop existing elements.
/// Furthermore, these core types can assume that all existing elements have
/// indices lower than the `used_len`.
pub trait Core<T> {
    /// Creates an instance without any elements. Must not allocate memory.
    fn new() -> Self;

    /// Creates an instance with the elements from `vec`. The returned instance
    /// has at least `vec.len()` capacity, and `used_len()` has to be
    /// `vec.len()` exactly.
    fn from_vec(vec: Vec<T>) -> Self;

    /// Returns the length of the range of elements that at one point were
    /// non-deleted (starting from 0). For a normal `Vec<T>`, this would be
    /// equivalent to `len()`.
    ///
    /// Specifically:
    /// - If the stable vector is compact, this equals the number of existing
    ///   elements
    /// - This is the index returned by the next `push()` call
    /// - This is one higher than the index of the element with highest index
    ///   that ever existed.
    fn used_len(&self) -> usize;

    /// Sets the `used_len` to a new value. In some methods (like `insert_at`),
    /// `used_len` is not automatically updated, so this method has to be used.
    ///
    /// # Safety
    /// - The new value must be `< self.capacity()`
    /// - All existing elements must have indices smaller than `v`
    ///
    /// If any of those condition is violated, the behavior of this method is
    /// undefined.
    unsafe fn set_used_len(&mut self, v: usize);

    /// Returns the number of elements that could be stored with the currently
    /// allocated memory.
    ///
    /// This is a bit different from `Vec::capacity` as all elements up to the
    /// capacity can be "used" with `has_element_at` and `insert_at` (all
    /// elements with indices ≥ `used_len()` are deleted).
    fn capacity(&self) -> usize;

    /// Reserves capacity for at least `additional` more elements to be added.
    ///
    /// This means that after calling this method, inserting elements at
    /// indices in the range `len()..len() + additional` is valid. `capacity`
    /// is equal to `len() + additional` after this call. If there is already
    /// enough memory allocated, this method must do nothing.
    fn grow(&mut self, additional: usize);

    /// Checks if there exists an element with index `idx`.
    ///
    /// # Safety
    /// - `idx` must be less than `self.capacity()`
    ///
    /// Otherwise, this method exhibits undefined behavior!
    unsafe fn has_element_at(&self, idx: usize) -> bool;

    /// Inserts `elem` at the index `idx`. Does *not* updated the `used_len`.
    ///
    /// # Safety
    /// - `idx` must be less than `self.capacity()`
    /// - `self.has_element_at(idx)` must be `false`
    ///
    /// Otherwise, this method exhibits undefined behavior!
    unsafe fn insert_at(&mut self, idx: usize, elem: T);

    /// Removes the element at index `idx` and returns it.
    ///
    /// # Safety
    /// - `idx` must be less than `self.capacity()`
    /// - `self.has_element_at(idx)` must be `true`
    ///
    /// Otherwise, this method exhibits undefined behavior!
    unsafe fn remove_at(&mut self, idx: usize) -> T;

    /// Returns a reference to the element at the index `idx`.
    ///
    /// # Safety
    /// - `idx` must be less than `self.capacity()`
    /// - `self.has_element_at(idx)` must be `true`
    ///
    /// Otherwise, this method exhibits undefined behavior!
    unsafe fn get_unchecked(&self, idx: usize) -> &T;

    /// Returns a mutable reference to the element at the index `idx`.
    ///
    /// # Safety
    /// - `idx` must be less than `self.capacity()`
    /// - `self.has_element_at(idx)` must be `true`
    ///
    /// Otherwise, this method exhibits undefined behavior!
    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T;

    /// Deletes all elements without deallocating memory. Drops all existing
    /// elements.
    ///
    /// After this call:
    /// - The capacity is the same.
    /// - `used_len` is 0
    fn clear(&mut self);

    /// Returns the index of the next non-deleted element with index `idx` or
    /// higher. Specifically, if an element at index `idx` exists, `Some(idx)`
    /// is returned. `idx` must be < `self.len`!
    fn next_index_from(&self, idx: usize) -> Option<usize>;

    /// Returns the index of the previous non-deleted element with index `idx`
    /// or lower. Specifically, if an element at index `idx` exists,
    /// `Some(idx)` is returned. `idx` must be < `self.len`!
    fn prev_index_from(&self, idx: usize) -> Option<usize>;
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
