# Changelog

## [0.8.1](https://github.com/pszymkowiak/diskstat/compare/v0.8.0...v0.8.1) (2026-03-13)


### Bug Fixes

* make subtree rescan non-blocking (async thread) ([#15](https://github.com/pszymkowiak/diskstat/issues/15)) ([5aea80a](https://github.com/pszymkowiak/diskstat/commit/5aea80abbf494ca19829e3a7f6f916a9c7ea29c3))

## [0.8.0](https://github.com/pszymkowiak/diskstat/compare/v0.7.0...v0.8.0) (2026-03-13)


### Features

* add 11 new languages (DE, ES, IT, PT, NL, PL, SV, RU, JA, ZH, KO) ([e2177c6](https://github.com/pszymkowiak/diskstat/commit/e2177c6e13722ba0f205c1f5746986fa9929b48a))
* add animated spinners for duplicate scan and delete operations ([b93bbca](https://github.com/pszymkowiak/diskstat/commit/b93bbcaea6ac24a68765d80abba64d27e7fef48d))
* add mouse click support for file tree navigation ([57e8706](https://github.com/pszymkowiak/diskstat/commit/57e87065145ff3c2e726e3f5ef76544056db7d1a))
* auto-launch duplicate scan in background with SQLite cache ([2d4d587](https://github.com/pszymkowiak/diskstat/commit/2d4d5875f6104144ef9cb09e600021486eb21a16))
* improve duplicates tab UX with file-level navigation and actions ([#14](https://github.com/pszymkowiak/diskstat/issues/14)) ([0fe1877](https://github.com/pszymkowiak/diskstat/commit/0fe1877d9746ddc5105e1f557c00d7235868c981))


### Bug Fixes

* force UI redraw during background operations (delete/scan/dupes) ([#12](https://github.com/pszymkowiak/diskstat/issues/12)) ([d7662a5](https://github.com/pszymkowiak/diskstat/commit/d7662a519215d0a3b3a3659d1e8393660f4f1cdd))
* improve spacing between tree and treemap panels ([9dd17aa](https://github.com/pszymkowiak/diskstat/commit/9dd17aaa0efea6e4ab092b070d61b37ea1b04454))

## [0.7.0](https://github.com/pszymkowiak/diskstat/compare/v0.6.0...v0.7.0) (2026-03-12)


### Features

* add size filtering (show only files/dirs ≥ threshold) ([4803d26](https://github.com/pszymkowiak/diskstat/commit/4803d26974b683d774339791fab84b867c25797d))


### Bug Fixes

* add market research step + web tools to improver agent ([d0d0423](https://github.com/pszymkowiak/diskstat/commit/d0d0423fc2ab1b8be2f8435dd2aedc2707bc648b))

## [0.6.0](https://github.com/pszymkowiak/diskstat/compare/v0.5.0...v0.6.0) (2026-03-12)


### Features

* add JSON export, config file support, and progress indicator ([b86246d](https://github.com/pszymkowiak/diskstat/commit/b86246dd3f2c1dfeed7bf8c2ed7479276d951056))

## [0.5.0](https://github.com/pszymkowiak/diskstat/compare/v0.4.0...v0.5.0) (2026-03-12)


### Features

* add improver agent + CLAUDE.md project instructions ([8b113f0](https://github.com/pszymkowiak/diskstat/commit/8b113f0889d4b49af8d3e7f82b6a85777b71afad))
* add sort modes and comprehensive tests ([cd154d6](https://github.com/pszymkowiak/diskstat/commit/cd154d6a7022da80d61d1fc5ad2f6a4d1f553cb0))

## [0.4.0](https://github.com/pszymkowiak/diskstat/compare/v0.3.0...v0.4.0) (2026-03-12)


### Features

* exclude patterns, top largest files view, file age display ([7afc70f](https://github.com/pszymkowiak/diskstat/commit/7afc70f9abf6949708165c09ded4202dd59e3aaa))

## [0.3.0](https://github.com/pszymkowiak/diskstat/compare/v0.2.0...v0.3.0) (2026-03-12)


### Features

* i18n (EN/FR), code dedup, doc comments, README architecture section ([8200916](https://github.com/pszymkowiak/diskstat/commit/82009166b8d9a1e9e678c156c013dbfcd1cb84f8))

## [0.2.0](https://github.com/pszymkowiak/diskstat/compare/v0.1.0...v0.2.0) (2026-03-12)


### Features

* bug fixes, CLI options, README multi-langue, unit tests ([7f33417](https://github.com/pszymkowiak/diskstat/commit/7f33417ed44fbae388776af5f7ff0c37dc495766))

## 0.1.0 (2026-03-12)


### Features

* initial commit — WinDirStat clone in Rust TUI ([0d7d79a](https://github.com/pszymkowiak/diskstat/commit/0d7d79a8cbd97215b518fc4823f7abdcb74a34d8))
* perf iteration 2 — extension interning, treemap single-pass, zero-copy render ([ed47894](https://github.com/pszymkowiak/diskstat/commit/ed47894f262f9cd76b0ec6808a64b8b1e3c65b5d))
* perf iteration 3 — symlink safety, OOM protection, search + render optim ([cdf5ec2](https://github.com/pszymkowiak/diskstat/commit/cdf5ec290c2169126da8f5156e7e168fa8d30c12))
* performance optimizations + release pipeline ([a346eae](https://github.com/pszymkowiak/diskstat/commit/a346eaeaa8d8cd96679d80f5d7d18935beec74a5))
