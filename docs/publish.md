# Publish

As of writing this document, I([baehyunsol]) am the only one who can publish a new version, and there's no plan to let others publish versions. This document is to 1) prevent myself from making mistakes and 2) inform the others what I do when I publish a new version.

Let's say the latest published version is 0.3.1, and you want to publish 0.3.2.

## Before publish

1. Run `tests/tests.py all`.
  - Run on linux, mac, windows.
  - Please make sure that `tests/migrate.py` and `tests/migrate2.py` are looking at the newest version, so that the new version does not cause a compatibility issue.
  - Sometimes I publish a new version even when there's a failing test. Those cases are 1) not that critical and 2) takes too long to fix.
2. Write release note before release (see `RelNotes/` for examples).
  - `git diff v0.3.1 main > release.diff` to see the differences.
  - If there's a compatilibity issue, please stop the publish process.
3. Search for "TODO" and "FIXME" in the entire repo.
  - If there's a serious issue, please stop the publish process.
4. Bump versions in `lib.rs`.
  - You'll find `pub const VERSION: &str = "0.3.2-dev";` in the file. Please change that to `"0.3.2"`.
  - You don't have to edit `Cargo.toml` files. You'll have to edit them after the publish.
5. Run `git commit`. The commit title must be `release 0.3.2`.
6. Run `cargo publish` in all the crates.
  - In order to avoid dependency issues, I recommend you to publish in this order: fs -> ignore -> korean -> pdl -> api -> cli -> core -> server.
7. Run `git push`.
  - Don't push until you publish all. If something goes wrong in step 6, do something and run `git commit --amend` before you push.

## After publish

1. Run `git tag v0.3.2` and `git push origin tag v0.3.2`.
2. Push knowledge-bases to ragit-server on `ragit.baehyunsol.com`, if there are updates.
3. Bump versions in `lib.rs` and `Cargo.toml`.
  - Change `pub const VERSION: &str = "0.3.2";` in `lib.rs` to `"0.3.3-dev";`.
  - Change all the versions in `Cargo.toml`s from `0.3.2` to `0.3.3`.
  - TODO: There must be a rule for when to increment patch version and when to increment minor version.
4. Run `git commit`. The commit title must be `bump versions`.
5. Add github release. The title must be `Version 0.3.2`.
  - Add binaries: make sure to checkout the correct commit, and make sure to run `cargo build --profile=production`.

[baehyunsol]: https://github.com/baehyunsol
