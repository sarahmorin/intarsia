# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha] - 2026-03-04

### Added

- Initial alpha release of intarsia-macros
- `isle_extractor!` - Generate extractor functions for ISLE operators
- `isle_constructor!` - Generate constructor functions for ISLE operators
- `isle_accessors!` - Generate both extractors and constructors for ISLE operators
- `isle_multi_extractor!` - Generate multi-extractor functions for exhaustive pattern matching
- `isle_multi_constructor!` - Generate multi-constructor functions with returns protocol
- `isle_multi_accessors!` - Generate both multi-extractors and multi-constructors with associated types
- `isle_integration!` - Generate type definitions required by ISLE-generated code
- `isle_integration_full!` - Complete ISLE integration with module declaration and type definitions
- Procedural macros for seamless ISLE DSL integration with Intarsia optimizer framework

[Unreleased]: https://github.com/sarahmorin/intarsia/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/sarahmorin/intarsia/releases/tag/v0.1.0-alpha
