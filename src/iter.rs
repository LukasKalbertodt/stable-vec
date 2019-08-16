//! Contains all iterator types and implementations.
//!
//! This is in its own module to not pollute the top-level namespace.

use std::{
    iter::FusedIterator,
    ops::Range,
};

use crate::{
    StableVecFacade,
    core::{Core, OwningCore},
};


/// Iterator over immutable references to a stable vec's elements and their
/// indices.
///
/// Use the method [`StableVecFacade::iter`] or the `IntoIterator` impl of
/// `&StableVecFacade` to obtain an iterator of this kind.
#[derive(Clone, Debug)]
pub struct Iter<'a, T, C: Core<T>>(Indices<'a, T, C>);

impl<'a, T, C: Core<T>> Iter<'a, T, C> {
    pub(crate) fn new(sv: &'a StableVecFacade<T, C>) -> Self {
        Self(Indices::new(sv))
    }
}

impl<'a, T, C: Core<T>> Iterator for Iter<'a, T, C> {
    type Item = (usize, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|idx| (idx, unsafe { self.0.core.get_unchecked(idx) }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, C: Core<T>> DoubleEndedIterator for Iter<'_, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|idx| (idx, unsafe { self.0.core.get_unchecked(idx) }))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Iter<'_, T, C> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T, C: Core<T>> FusedIterator for Iter<'_, T, C> {}


/// Iterator over mutable references to a stable vec's elements and their
/// indices.
///
/// Use the method [`StableVecFacade::iter_mut`] or the `IntoIterator` impl of
/// `&mut StableVecFacade` to obtain an iterator of this kind.
#[derive(Debug)]
pub struct IterMut<'a, T, C: Core<T>> {
    pub(crate) core: &'a mut OwningCore<T, C>,
    pub(crate) remaining: Range<usize>,
    pub(crate) count: usize,
}

impl<'a, T, C: Core<T>> IterMut<'a, T, C> {
    pub(crate) fn new(sv: &'a mut StableVecFacade<T, C>) -> Self {
        Self {
            remaining: 0..sv.core.len(),
            core: &mut sv.core,
            count: sv.num_elements,
        }
    }
}

impl<'a, T, C: Core<T>> Iterator for IterMut<'a, T, C> {
    type Item = (usize, &'a mut T);
    fn next(&mut self) -> Option<Self::Item> {
        next(&mut self.count, &mut self.remaining, &**self.core).map(|idx| {
            // This is... scary. We are extending the lifetime of the reference
            // returned by `get_unchecked_mut`. We can do that because we know
            // that we will never return the same reference twice. So the user
            // can't have mutable aliases. Furthermore, all access to the
            // original stable vector is blocked because we (`ValuesMut`) have
            // a mutable reference to it. So it is fine to extend the lifetime
            // to `'a`.
            let r = unsafe { &mut *(self.core.get_unchecked_mut(idx) as *mut T) };
            (idx, r)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }

    fn count(self) -> usize {
        self.len()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, C: Core<T>> DoubleEndedIterator for IterMut<'_, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        next_back(&mut self.count, &mut self.remaining, &**self.core).map(|idx| {
            // See `Self::next()` for more information on this.
            let r = unsafe { &mut *(self.core.get_unchecked_mut(idx) as *mut T) };
            (idx, r)
        })
    }
}

impl<T, C: Core<T>> ExactSizeIterator for IterMut<'_, T, C> {
    fn len(&self) -> usize {
        self.count
    }
}

impl<T, C: Core<T>> FusedIterator for IterMut<'_, T, C> {}


/// Iterator over immutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::values`] to obtain an iterator of this
/// kind.
#[derive(Clone, Debug)]
pub struct Values<'a, T, C: Core<T>>(Indices<'a, T, C>);

impl<'a, T, C: Core<T>> Values<'a, T, C> {
    pub(crate) fn new(sv: &'a StableVecFacade<T, C>) -> Self {
        Self(Indices::new(sv))
    }
}

impl<'a, T, C: Core<T>> Iterator for Values<'a, T, C> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|idx| unsafe { self.0.core.get_unchecked(idx) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, C: Core<T>> DoubleEndedIterator for Values<'_, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|idx| unsafe { self.0.core.get_unchecked(idx) })
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Values<'_, T, C> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T, C: Core<T>> FusedIterator for Values<'_, T, C> {}


/// Iterator over mutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::values_mut`] to obtain an iterator of
/// this kind.
#[derive(Debug)]
pub struct ValuesMut<'a, T, C: Core<T>>(IterMut<'a, T, C>);

impl<'a, T, C: Core<T>> ValuesMut<'a, T, C> {
    pub(crate) fn new(sv: &'a mut StableVecFacade<T, C>) -> Self {
        Self(IterMut::new(sv))
    }
}

impl<'a, T, C: Core<T>> Iterator for ValuesMut<'a, T, C> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, r)| r)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, C: Core<T>> DoubleEndedIterator for ValuesMut<'_, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(_, r)| r)
    }
}

impl<T, C: Core<T>> ExactSizeIterator for ValuesMut<'_, T, C> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T, C: Core<T>> FusedIterator for ValuesMut<'_, T, C> {}


/// Iterator over owned elements of a `StableVecFacade`.
///
/// Use the method `StableVecFacade::into_iter` to obtain an iterator of this
/// kind.
#[derive(Debug)]
pub struct IntoIter<T, C: Core<T>> {
    pub(crate) sv: StableVecFacade<T, C>,
    pub(crate) pos: usize,
}

impl<T, C: Core<T>> Iterator for IntoIter<T, C> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.sv.core.first_filled_slot_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.sv.num_elements -= 1;
            let elem = unsafe {
                // We know that `idx` is a valid.
                self.sv.core.remove_at(idx)
            };

            Some(elem)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.sv.num_elements, Some(self.sv.num_elements))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for IntoIter<T, C> {}


/// Iterator over all indices of filled slots of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::indices`] to obtain an iterator of this
/// kind.
#[derive(Clone, Debug)]
pub struct Indices<'a, T, C: Core<T>> {
    core: &'a OwningCore<T, C>,
    remaining: Range<usize>,
    count: usize,
}

impl<'a, T, C: Core<T>> Indices<'a, T, C> {
    pub(crate) fn new(sv: &'a StableVecFacade<T, C>) -> Self {
        Self {
            core: &sv.core,
            remaining: 0..sv.core.len(),
            count: sv.num_elements,
        }
    }
}

impl<T, C: Core<T>> Iterator for Indices<'_, T, C> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        next(&mut self.count, &mut self.remaining, &**self.core)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }

    fn count(self) -> usize {
        self.len()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, C: Core<T>> DoubleEndedIterator for Indices<'_, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        next_back(&mut self.count, &mut self.remaining, &**self.core)
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Indices<'_, T, C> {
    fn len(&self) -> usize {
        self.count
    }
}

impl<T, C: Core<T>> FusedIterator for Indices<'_, T, C> {}


/// The actual logic for all `next()` iterator methods.
fn next<T, C: Core<T>>(
    count: &mut usize,
    remaining: &mut Range<usize>,
    core: &C,
) -> Option<usize> {
    if *count == 0 {
        return None;
    }

    let idx = unsafe { core.first_filled_slot_from(remaining.start) }
        .expect("bug in StableVec iterator: no next filled slot");

    remaining.start = idx + 1;
    *count -= 1;

    Some(idx)
}

/// The actual logic for all `next_back()` iterator methods.
fn next_back<T, C: Core<T>>(
    count: &mut usize,
    remaining: &mut Range<usize>,
    core: &C,
) -> Option<usize> {
    if *count == 0 {
        return None;
    }

    let idx = unsafe { core.first_filled_slot_below(remaining.end) }
        .expect("bug in StableVec iterator: no next filled slot");

    remaining.end = idx;
    *count -= 1;

    Some(idx)
}
