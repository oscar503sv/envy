//! Fake secret marker. Any test output, Debug, or Display containing it is a leak.

pub(crate) const SENTINEL: &str = "ENVY_TEST_SENTINEL_9F2C";

#[path = "../tests/common/mod.rs"]
pub(crate) mod diff;
