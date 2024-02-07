# Changelog

# v0.1.5

bump dependencies

# v0.1.4

lib: replace find_file() with a small specialized function. See https://github.com/bim9262/calibright/pull/11
license: Add symlink to LICENSE file in lib and bin dirs. See https://github.com/bim9262/calibright/pull/12

# v0.1.3

config: Do not look for files relative to the CWD. See https://github.com/greshake/i3status-rust/issues/1870.
deps: Do not depend on env_logger. See https://github.com/bim9262/calibright/pull/7
deps: Bump enumflags2 from 0.7.5 to 0.7.7. See https://github.com/bim9262/calibright/pull/8
deps: Update dirs, smart-default, and zbus. See https://github.com/bim9262/calibright/pull/10
debug: Update debug names to make sure they mention calibright

# v0.1.2

docs: Add documentation for public functions/fields.
    Use `#[cfg_attr(docsrs, doc(cfg(feature = "watch")))]` to mark what's behind a feature flag.

# v0.1.1

Initial release