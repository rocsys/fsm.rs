# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2021-03-30

### Added

- Add an `FsmArc` with the state object to each variant of state enums.
- Add an `FsmRetrieveStateName` trait that allows retrieving the fully-qualified name of states.

## [0.5.0] - 2021-02-25

### Added

- The ability to define an error state for machines with the `ErrorState` macro.

### Fixed

- The `start` function of submachines weren't being called when they are used as the initial state of their parent.

## [0.4.0] - 2021-02-23

### Changed

- Upgraded tokio to `v1.x`.

## [0.3.0] - 2021-02-23

### Added

- Allow sending events from sub to super states through the `process_event` function.

### Changed

- All associated functions in Fsm traits have been made immutable (i.e., `&self` instead of `&mut self`).

## [0.2.0] - 2020-12-09

### Added

- Support for `async/await` in state and transition actions.
- Expose a mutable context on the state machine via the `get_context_mut` function.

### Changed

- Update the lib to use 2018 edition of Rust.
- Reduce many compilation warnings.
