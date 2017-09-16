# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Added method overview in documentation


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


[Unreleased]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/stable-vec/compare/v0.1.0...v0.1.1
