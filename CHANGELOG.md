# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/carvalhosauro/ray-task/compare/v0.1.0...v0.1.1) - 2026-10-06

### Added

- *(app)* add menu shortcut on first launch and new icon ([#4](https://github.com/carvalhosauro/ray-task/pull/4))
- *(core)* add background writer with retry and bounded shutdown
- *(core)* add undo for delete, complete and move
- *(core)* add views, counters, due status and text filter
- *(core)* add in-memory store with domain rules
- *(core)* add SQLite schema, migrations, load and apply
- *(core)* add domain model, errors and clock

### Fixed

- *(core)* log writer send failures and unflushed queue on exit

### Other

- extract list logic into gus-list crates ([#5](https://github.com/carvalhosauro/ray-task/pull/5))
- prepare ray-task for open source ([#1](https://github.com/carvalhosauro/ray-task/pull/1))
- add MIT license
- add README and manual release checklist
- fix clippy warnings
- *(core)* add performance budget test for 5000 tasks
