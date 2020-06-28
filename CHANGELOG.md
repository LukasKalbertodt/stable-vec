# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.4.0] - 2019-08-26
This is a pretty large release. The whole crate was more or less completely
rewritten. So it might almost be more useful to forget everything about the
crate and learn everything anew instead of digging through this changelog.

### Added
- `Core` trait and two implementations
- Add `StableVec`, `ExternStableVec` and `InlineStableVec` type aliases for
  `StableVecFacade`
- `Iter` and `IterMut` which iterate over (index, reference) pairs
- All iterators implement `DoubleSidedIterator` and `FusedIterator`
- `StableVecFacade::{get_unchecked, get_unchecked_mut}`
- `StableVecFacade::{reserve_exact, reserve_for}`
- `StableVecFacade::{first_filled_slot_from, first_filled_slot_below}`
- `StableVecFacade::{first_empty_slot_from, first_empty_slot_below}`
- `StableVecFacade::swap`

### Changed
- Notable performance improvements crate-wide
- `StableVec` has been renamed to `StableVecFacade` and is generic over a
  "core" (parameter `C`)
- Move iterators into `iter` submodule
- The terminology in the documentation changed a bit (notably: "empty or filled
  slots" instead of "deleted or existing elements")
- `PartialEq<StableVecFacade>` not compares all observable properties
- Rename `Iter`/`IterMut` into `Values`/`ValuesMut`
- Rename `StableVecFacade::next_index` to `next_push_index`
- Rename `keys`/`Keys` to `indices`/`Indices`
- Replace `insert_into_hole` with `insert`
- The `IntoIter` now yields (index, value) pairs
- The crate `bit-vec` is no longer used and has been replaced by our own
  implementation
- The crate is now usable in `no_std` context (still requires `alloc` though)

### Removed
- `StableVecFacade::into_vec`
- `StableVecFacade::from_vec`
- `StableVecFacade::grow`
- `io::Write` impl for `StableVecFacade<u8>`


## [0.3.2] - 2019-04-09
### Added
- `remove_last`
- `remove_first`
- `find_first`
- `find_first_mut`
- `find_first_index`
- `find_last`
- `find_last_mut`
- `find_last_index`
- `retain_indices`

### Fixed
- Suboptimal use of `bit-vec` which lead to suboptimal generated assembly.
  Now the most important occurences of this are fixed.


## [0.3.1] - 2019-01-24
### Fixed
- Fix memory safety bug in `clone()`: cloning a non-compact stable vec before
  this change accessed already dropped values and attempted to clone them.
- Fix memory safety bug in `clear()`: calling `clear()` on a non-compact
  stable vec before would access already dropped values and drop them a second
  time.

## [0.3.0] - 2019-01-07
### Removed
- Remove `IterMut::remove_current`. The method was broken because it did not
  drop the removed value properly. On closer inspection, the method was a bit
  broken in some other minor other ways because it was badly designed. For now
  it is just removed and may reappear with good design in the future.

## [0.2.2] - 2019-01-07
### Added
- All three iterators implement `Iterator::size_hint` and `ExactSizeIterator`
  now and report the correct length.

## [0.2.1] - 2018-09-26
### Added
- `StableVec::insert_into_hole()`
- `StableVec::grow()`
- `StableVec::clear()`
- `StableVec::from_vec()`
- `StableVec::extend_from_slice()`
- `Debug` implementations for `Iter`, `IterMut` and `Keys`
- `Write` implementation for `StableVec<u8>`

### Changed
- The `Drop` impl now uses the `mem::needs_drop()` optimization hint to avoid
  unnecessary overhead.
- Updated `bit-vec` from `0.4` to `0.5`

## [0.2.0] - 2017-09-17
### Added
- Added method overview in documentation
- `StableVec::contains()`
- `StableVec::into_vec()`
- `StableVec::retain()`
- `StableVec::make_compact()`
- `StableVec::keys()` with `Keys` iterator
- `IterMut::remove_current()`
- `impl<T> Default for StableVec<T>`
- Added `FromIterator` impl for `StableVec`
- Added `Extend` impl for `StableVec`
- Added `Debug` impl for `StableVec` with a fitting example
- Added `PartialEq` impls to compare a `StableVec` with slices and `Vec`s

### Changed
- Renamed `compact()` to `reordering_make_compact()`: changing element order by
  default is a bad idea. Instead `make_compact()` should be used to preserve
  order.
- Renamed `exists()` to `has_element_at()`

## [0.1.2] - 2017-09-15
### Fixed
- Travis-CI badge entry in `Cargo.toml`
- Warning in example

## [0.1.1] - 2017-09-15
### Added
- Added this `CHANGELOG.md`
- Metadata to `Cargo.toml`

### Changed
- Documentation examples are now clearer by calling variables storing indices
  `_idx` at the end

### Fixed
- Fixed panic in the `Iter` iterator impl which occured when the last element
  was removed from the vector before

## 0.1.0 - 2017-06-21
### Added
- Everything.


[Unreleased]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.0...v0.1.1
