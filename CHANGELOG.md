# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2019-10-07
### Added
- Key confirmation using HKDF from [key_confirmation](https://github.com/landhb/portal/tree/key_confirmation)
- Relay efficiency improvements from [multiproc_relay](https://github.com/landhb/portal/tree/multiproc_relay)

### Changed
- The main Portal struct's direction field is now a `Direction` instead of an `Option<Direction>`, this is a breaking change since 0.1.0 clients won't be able to communicate with a 0.2.0 relay

### Fixed
- Library Documentation

## [0.1.0] - 2019-10-02
### Added
- Initial publication


[0.2.0]: 
[0.1.0]: https://github.com/landhb/portal/tree/b228f9a8d0e765c1f4f2f37799df5d55483dfece