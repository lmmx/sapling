import ".just/cargo.just"
import ".just/commit.just"
import ".just/hooks.just"
import ".just/release.just"
import ".just/release-py.just"

default: pc-fix clippy doctest test
