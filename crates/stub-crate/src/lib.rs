//! Stub crate for the initial Starbase repository setup.

pub fn stub_message() -> &'static str {
    "Starbase stub crate is wired."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_stub_message() {
        assert_eq!(stub_message(), "Starbase stub crate is wired.");
    }
}
