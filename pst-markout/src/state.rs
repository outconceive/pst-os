use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum StateValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl StateValue {
    pub fn as_text(&self) -> Option<&str> {
        match self { StateValue::Text(s) => Some(s), _ => None }
    }
    pub fn as_number(&self) -> Option<f64> {
        match self { StateValue::Number(n) => Some(*n), _ => None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self { StateValue::Bool(b) => Some(*b), _ => None }
    }
    pub fn display(&self) -> String {
        match self {
            StateValue::Text(s) => s.clone(),
            StateValue::Number(n) => {
                if *n == (*n as i64) as f64 { format!("{}", *n as i64) }
                else { format!("{:.2}", n) }
            }
            StateValue::Bool(b) => String::from(if *b { "true" } else { "false" }),
            StateValue::Null => String::new(),
        }
    }
}

#[derive(Debug)]
pub struct StateStore {
    values: BTreeMap<String, StateValue>,
    dirty_keys: BTreeSet<String>,
}

impl StateStore {
    pub fn new() -> Self {
        Self { values: BTreeMap::new(), dirty_keys: BTreeSet::new() }
    }

    pub fn get(&self, key: &str) -> Option<&StateValue> {
        self.values.get(key)
    }

    pub fn get_text(&self, key: &str) -> String {
        self.values.get(key).map(|v| v.display()).unwrap_or_default()
    }

    pub fn get_number(&self, key: &str) -> f64 {
        self.values.get(key).and_then(|v| v.as_number()).unwrap_or(0.0)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.values.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub fn set(&mut self, key: &str, value: StateValue) {
        let changed = self.values.get(key) != Some(&value);
        if changed {
            self.values.insert(String::from(key), value);
            self.dirty_keys.insert(String::from(key));
        }
    }

    pub fn set_text(&mut self, key: &str, value: &str) {
        self.set(key, StateValue::Text(String::from(value)));
    }

    pub fn set_number(&mut self, key: &str, value: f64) {
        self.set(key, StateValue::Number(value));
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, StateValue::Bool(value));
    }

    pub fn toggle(&mut self, key: &str) {
        let current = self.get_bool(key);
        self.set_bool(key, !current);
    }

    pub fn get_list_count(&self, list_key: &str) -> usize {
        self.values.get(&format!("{}._count", list_key))
            .and_then(|v| v.as_number())
            .map(|n| n as usize)
            .unwrap_or(0)
    }

    pub fn add_list_item(&mut self, list_key: &str, fields: &[(String, StateValue)]) {
        let count = self.get_list_count(list_key);
        for (field, value) in fields {
            let key = format!("{}.{}.{}", list_key, count, field);
            self.set(&key, value.clone());
        }
        self.set(&format!("{}._count", list_key), StateValue::Number((count + 1) as f64));
        self.dirty_keys.insert(String::from(list_key));
    }

    pub fn remove_list_item(&mut self, list_key: &str, index: usize) {
        let count = self.get_list_count(list_key);
        if index >= count { return; }

        for i in index..count - 1 {
            let next_prefix = format!("{}.{}.", list_key, i + 1);
            let next_keys: Vec<(String, StateValue)> = self.values.iter()
                .filter(|(k, _)| k.starts_with(&next_prefix))
                .map(|(k, v)| (String::from(&k[next_prefix.len()..]), v.clone()))
                .collect();

            let cur_prefix = format!("{}.{}.", list_key, i);
            let to_remove: Vec<String> = self.values.keys()
                .filter(|k| k.starts_with(&cur_prefix))
                .cloned().collect();
            for k in to_remove { self.values.remove(&k); }

            for (field, value) in next_keys {
                self.values.insert(format!("{}.{}.{}", list_key, i, field), value);
            }
        }

        let last_prefix = format!("{}.{}.", list_key, count - 1);
        let to_remove: Vec<String> = self.values.keys()
            .filter(|k| k.starts_with(&last_prefix))
            .cloned().collect();
        for k in to_remove { self.values.remove(&k); }

        self.set(&format!("{}._count", list_key), StateValue::Number((count - 1) as f64));
        self.dirty_keys.insert(String::from(list_key));
    }

    pub fn get_scoped(&self, scope: &str, key: &str) -> Option<&StateValue> {
        self.values.get(&format!("{}.{}", scope, key))
    }

    pub fn get_scoped_text(&self, scope: &str, key: &str) -> String {
        self.get_scoped(scope, key).map(|v| v.display()).unwrap_or_default()
    }

    pub fn take_dirty_keys(&mut self) -> BTreeSet<String> {
        core::mem::take(&mut self.dirty_keys)
    }

    pub fn has_dirty_keys(&self) -> bool {
        !self.dirty_keys.is_empty()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_state() {
        let mut s = StateStore::new();
        s.set_text("name", "Alice");
        assert_eq!(s.get_text("name"), "Alice");
    }

    #[test]
    fn test_bool_toggle() {
        let mut s = StateStore::new();
        assert_eq!(s.get_bool("agree"), false);
        s.toggle("agree");
        assert_eq!(s.get_bool("agree"), true);
        s.toggle("agree");
        assert_eq!(s.get_bool("agree"), false);
    }

    #[test]
    fn test_number() {
        let mut s = StateStore::new();
        s.set_number("count", 42.0);
        assert_eq!(s.get_number("count"), 42.0);
        assert_eq!(s.get_text("count"), "42");
    }

    #[test]
    fn test_dirty_tracking() {
        let mut s = StateStore::new();
        s.set_text("a", "hello");
        assert!(s.has_dirty_keys());
        let dirty = s.take_dirty_keys();
        assert!(dirty.contains("a"));
        assert!(!s.has_dirty_keys());
    }

    #[test]
    fn test_list_add_remove() {
        let mut s = StateStore::new();
        s.add_list_item("items", &[
            (String::from("name"), StateValue::Text(String::from("Apple"))),
        ]);
        s.add_list_item("items", &[
            (String::from("name"), StateValue::Text(String::from("Banana"))),
        ]);
        assert_eq!(s.get_list_count("items"), 2);
        assert_eq!(s.get_scoped_text("items.0", "name"), "Apple");
        assert_eq!(s.get_scoped_text("items.1", "name"), "Banana");

        s.remove_list_item("items", 0);
        assert_eq!(s.get_list_count("items"), 1);
        assert_eq!(s.get_scoped_text("items.0", "name"), "Banana");
    }

    #[test]
    fn test_no_dirty_on_same_value() {
        let mut s = StateStore::new();
        s.set_text("x", "hello");
        s.take_dirty_keys();
        s.set_text("x", "hello"); // same value
        assert!(!s.has_dirty_keys());
    }
}
