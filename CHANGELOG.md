# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - TBD

### Added
- Multi-file send support.
- Ring backend support.

### Changed
- Refactored API into higher/lower level abstractions. Not backwards compat.
- Refactored Client code to use newer API
- Refactored Relay code to use newer API

### Fixed
- N/A

## [0.3.0] - 2022-03-27

### Added
- Library Benchmarks in [PR8](https://github.com/landhb/portal/pull/8)

### Changed
- Metadata is now encrypted in [PR5](https://github.com/landhb/portal/pull/5)
- Improved Relay logging in [PR7](https://github.com/landhb/portal/pull/7)
- Changed signature of sync_file_state() method to not take a copy/ownership in [PR8](https://github.com/landhb/portal/pull/8). Required for benchmarking. And improves the ergonomics of the API.

### Fixed
- Removed unecessary write in PortalFile::download_file() [PR9](https://github.com/landhb/portal/pull/9).

## [0.2.0] - 2020-10-07
### Added
- Key confirmation using HKDF 
- Relay efficiency improvements, polling only for read events bi-directionally until the intermediary pipe needs to be drained (i.e request exchange or the end of the transfer)

### Changed
- The main Portal struct's direction field is now a `Direction` instead of an `Option<Direction>`, this is a breaking change since 0.1.0 clients won't be able to communicate with a 0.2.0 relay

### Fixed
- Library Documentation

## [0.1.0] - 2020-10-02
### Added
- Initial publication


## Source - All Releases

```
[0.2.0]: https://github.com/landhb/portal/releases/tag/v0.2
[0.1.0]: https://github.com/landhb/portal/tree/b228f9a8d0e765c1f4f2f37799df5d55483dfece
```
