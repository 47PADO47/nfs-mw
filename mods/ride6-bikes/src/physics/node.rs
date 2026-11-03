//! A cursor over the resolved JSON of a vehicle that remembers its path, so every error names the field it is about.

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

/// A value of the resolved JSON and the path that led to it (`Properties.FrontWheel.Mass`).
#[derive(Clone, Debug)]
pub struct Node<'a> {
    value: &'a Value,
    path: String,
}

impl<'a> Node<'a> {
    pub fn new(value: &'a Value, path: impl Into<String>) -> Self {
        Self { value, path: path.into() }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn value(&self) -> &'a Value {
        self.value
    }

    /// The member `key`; `None` when it is missing or JSON `null` (an unset object reference).
    pub fn get(&self, key: &str) -> Option<Node<'a>> {
        let child = self.value.get(key)?;
        if child.is_null() {
            return None;
        }
        Some(Node { value: child, path: format!("{}.{key}", self.path) })
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn req(&self, key: &str) -> Result<Node<'a>> {
        self.get(key).ok_or_else(|| anyhow!("{}: the field `{key}` is missing from the resolved data", self.path))
    }

    pub fn num(&self) -> Result<f64> {
        let Some(number) = self.value.as_f64() else { bail!("{}: not a number", self.path) };
        if !number.is_finite() {
            bail!("{}: not a finite number", self.path);
        }
        Ok(number)
    }

    pub fn flag(&self) -> Result<bool> {
        self.value.as_bool().ok_or_else(|| anyhow!("{}: not true or false", self.path))
    }

    pub fn text(&self) -> Result<&'a str> {
        self.value.as_str().ok_or_else(|| anyhow!("{}: not a string", self.path))
    }

    pub fn items(&self) -> Result<Vec<Node<'a>>> {
        let list = self.value.as_array().ok_or_else(|| anyhow!("{}: not a list", self.path))?;
        Ok(list
            .iter()
            .enumerate()
            .map(|(index, value)| Node { value, path: format!("{}[{index}]", self.path) })
            .collect())
    }

    /// An `FVector2D` (`{"X": .., "Y": ..}`).
    pub fn xy(&self) -> Result<[f64; 2]> {
        Ok([self.req("X")?.num()?, self.req("Y")?.num()?])
    }

    /// An `FVector` (`{"X": .., "Y": .., "Z": ..}`).
    pub fn xyz(&self) -> Result<[f64; 3]> {
        Ok([self.req("X")?.num()?, self.req("Y")?.num()?, self.req("Z")?.num()?])
    }

    /// The number at `key`.
    pub fn f(&self, key: &str) -> Result<f64> {
        self.req(key)?.num()
    }

    /// The number at `key`, or `default` when the asset leaves it out (a constructor default of the reference game: the
    /// caller names the document that gives it).
    pub fn f_or(&self, key: &str, default: f64) -> Result<f64> {
        match self.get(key) {
            Some(node) => node.num(),
            None => Ok(default),
        }
    }

    pub fn flag_or(&self, key: &str, default: bool) -> Result<bool> {
        match self.get(key) {
            Some(node) => node.flag(),
            None => Ok(default),
        }
    }

    pub fn xy_of(&self, key: &str) -> Result<[f64; 2]> {
        self.req(key)?.xy()
    }

    pub fn xy_or(&self, key: &str, default: [f64; 2]) -> Result<[f64; 2]> {
        match self.get(key) {
            Some(node) => node.xy(),
            None => Ok(default),
        }
    }

    pub fn xyz_or(&self, key: &str, default: [f64; 3]) -> Result<[f64; 3]> {
        match self.get(key) {
            Some(node) => node.xyz(),
            None => Ok(default),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_error_names_the_path() {
        let value = json!({"A": {"B": 2.5, "C": null, "V": {"X": 1.0, "Y": 2.0}}});
        let root = Node::new(&value, "Properties");
        let a = root.req("A").unwrap();
        assert_eq!(a.f("B").unwrap(), 2.5);
        assert!(!a.has("C"));
        assert_eq!(a.f_or("C", 7.0).unwrap(), 7.0);
        assert_eq!(a.xy_of("V").unwrap(), [1.0, 2.0]);
        let error = a.req("D").unwrap_err().to_string();
        assert!(error.contains("Properties.A") && error.contains("`D`"), "{error}");
    }
}
