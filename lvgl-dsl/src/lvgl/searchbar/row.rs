//! SearchRow — caller-owned data for one result row (§3.1).
use alloc::string::String;

#[derive(Clone, Debug)]
pub struct SearchRow {
    pub id: u64,
    pub primary: String,
    pub secondary: Option<String>,
    pub tertiary: Option<String>,
    pub disabled: bool,
}

impl SearchRow {
    pub fn new(id: u64, primary: impl Into<String>) -> Self {
        Self {
            id,
            primary: primary.into(),
            secondary: None,
            tertiary: None,
            disabled: false,
        }
    }
    pub fn with_secondary(mut self, s: impl Into<String>) -> Self {
        self.secondary = Some(s.into());
        self
    }
    pub fn with_tertiary(mut self, t: impl Into<String>) -> Self {
        self.tertiary = Some(t.into());
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_builder_defaults() {
        let r = SearchRow::new(1, "Pizza");
        assert_eq!(r.id, 1);
        assert_eq!(r.primary, "Pizza");
        assert!(r.secondary.is_none());
        assert!(!r.disabled);
    }
    #[test]
    fn row_builder_chain() {
        let r = SearchRow::new(2, "X").with_secondary("Y").disabled(true);
        assert_eq!(r.secondary.as_deref(), Some("Y"));
        assert!(r.disabled);
    }
    #[test]
    fn with_tertiary_sets_field() {
        let r = SearchRow::new(1, "Alice").with_tertiary("WC-001");
        assert_eq!(r.tertiary.as_deref(), Some("WC-001"));
    }
    #[test]
    fn tertiary_defaults_to_none() {
        let r = SearchRow::new(1, "Alice");
        assert!(r.tertiary.is_none());
    }
}
