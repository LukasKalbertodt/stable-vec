//! Contains all iterator types and implementations.
//!
//! This is in its own module to not pollute the top-level namespace.

use std::fmt;

use crate::{
    StableVecFacade,
    core::{Core, OwningCore},
};


/// Iterator over immutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::values`] to obtain an iterator of this
/// kind.
pub struct Values<'a, T, C: Core<T>> {
    pub(crate) core: &'a OwningCore<T, C>,
    pub(crate) pos: usize,
    pub(crate) count: usize,
}

impl<'a, T, C: Core<T>> Iterator for Values<'a, T, C> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.core.first_filled_slot_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.count -= 1;
        }

        idx.map(|idx| unsafe { self.core.get_unchecked(idx) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Values<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for Values<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Values")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}

impl<T, C: Core<T>> Clone for Values<'_, T, C> {
    fn clone(&self) -> Self {
        Self {
            core: self.core,
            pos: self.pos,
            count: self.count,
        }
    }
}


/// Iterator over mutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::values_mut`] to obtain an iterator of
/// this kind.
pub struct ValuesMut<'a, T, C: Core<T>> {
    pub(crate) sv: &'a mut StableVecFacade<T, C>,
    pub(crate) pos: usize,
    pub(crate) count: usize,
}

impl<'a, T, C: Core<T>> Iterator for ValuesMut<'a, T, C> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.sv.core.first_filled_slot_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.count -= 1;
        }

        // This is... scary. We are extending the lifetime of the reference
        // returned by `get_unchecked_mut`. We can do that because we know that
        // we will never return the same reference twice. So the user can't
        // have mutable aliases. Furthermore, all access to the original stable
        // vector is blocked because we (`ValuesMut`) have a mutable reference
        // to it. So it is fine to extend the lifetime to `'a`.
        idx.map(|idx| {
            unsafe { &mut *(self.sv.core.get_unchecked_mut(idx) as *mut T) }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for ValuesMut<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for ValuesMut<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ValuesMut")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}


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


/// Iterator over all valid indices of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::indices`] to obtain an iterator of this
/// kind.
pub struct Indices<'a, T, C: Core<T>> {
    pub(crate) core: &'a OwningCore<T, C>,
    pub(crate) pos: usize,
    pub(crate) count: usize,
}

impl<T, C: Core<T>> Iterator for Indices<'_, T, C> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let out = unsafe { self.core.first_filled_slot_from(self.pos) };
        if let Some(idx) = out {
            self.pos = idx + 1;
            self.count -= 1;
        }

        out
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Indices<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for Indices<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Indices")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}

impl<T, C: Core<T>> Clone for Indices<'_, T, C> {
    fn clone(&self) -> Self {
        Self {
            core: self.core,
            pos: self.pos,
            count: self.count,
        }
    }
}
