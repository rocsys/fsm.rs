# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
