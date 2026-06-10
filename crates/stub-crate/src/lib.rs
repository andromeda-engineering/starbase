//! Stub crate for the initial NeonOS repository setup.

pub fn stub_message() -> &'static str {
    "NeonOS stub crate is wired."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_stub_message() {
        assert_eq!(stub_message(), "NeonOS stub crate is wired.");
    }
}
