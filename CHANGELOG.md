# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.1.0-alpha] - 2026-03-04

### Added

- Initial alpha release of Intarsia optimizer framework
- Cascades-style optimization algorithm with property-aware search
- Generic `OptimizerFramework<L, P, C, UserData>` supporting any language and property system
- Property system via `Property` trait for defining semantic requirements
- `PropertyAwareLanguage` trait for specifying operator property requirements
- Cost model with `CostDomain` and `CostFunction` traits
- `SimpleCost<P>` implementation for basic cost modeling
- `Task` system for managing optimization workflow
- `ExplorerHooks` trait for integrating rewrite rules
- ISLE integration support via intarsia-macros
- E-graph backend powered by egg
- Support for user-defined data accessible during optimization
- Comprehensive documentation and examples
- `SimpleOptimizerFramework<L, P>` type alias for easy setup
- `build-helpers` feature flag for re-exporting build utilities

### Examples

- Boolean optimizer demonstrating basic rewrite rules
- Database query optimizer with property-aware join ordering

<!-- next-url -->
[Unreleased]: https://github.com/sarahmorin/intarsia/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/sarahmorin/intarsia/releases/tag/v0.1.0-alpha
