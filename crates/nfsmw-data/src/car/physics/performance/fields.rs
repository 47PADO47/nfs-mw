//! Immutable numeric attribute view. See docs/specs/performance-upgrades.md.

use super::super::Fields;
use blackbox_attrib::{CollectionRef, Value, vlt_hash};

pub trait PerformanceFields {
    fn floats(&self, name: &str) -> Vec<f32>;
    fn f32(&self, name: &str) -> f32 {
        self.floats(name).first().copied().unwrap_or(0.0)
    }
    fn axle_pair(&self, name: &str) -> [f32; 2] {
        let values = self.floats(name);
        match values.as_slice() {
            [] => [0.0; 2],
            [v] => [*v; 2],
            [a, b, ..] => [*a, *b],
        }
    }
    fn pair(&self, name: &str) -> [f32; 2] {
        self.axle_pair(name)
    }
}

impl PerformanceFields for Fields<'_> {
    fn floats(&self, name: &str) -> Vec<f32> {
        numbers(self.0.get(name))
    }
}

impl<T: PerformanceFields + ?Sized> PerformanceFields for &T {
    fn floats(&self, name: &str) -> Vec<f32> {
        (**self).floats(name)
    }
}

pub(super) struct ResolvedFields<'a> {
    pub low: CollectionRef<'a>,
    pub high: Option<CollectionRef<'a>>,
    pub fraction: f32,
    pub modifiers: Vec<(u32, f32)>,
}

impl PerformanceFields for ResolvedFields<'_> {
    fn floats(&self, name: &str) -> Vec<f32> {
        let lower = self.low.get(name);
        let upper = self.high.and_then(|c| c.get(name));
        let mut values = numbers(lower);
        if let Some(upper) = upper
            && same_type(lower, upper)
        {
            values = blend(&values, &numbers(Some(upper)), self.fraction);
        }
        let integer = lower.and_then(first).is_some_and(|v| v.as_i64().is_some());
        if integer {
            values.iter_mut().for_each(|v| *v = v.trunc());
        }
        for &(_, scale) in self.modifiers.iter().filter(|(key, _)| *key == vlt_hash(name)) {
            for value in &mut values {
                *value *= scale;
                if integer {
                    *value = value.trunc();
                }
            }
        }
        values
    }
}

fn first(value: &Value) -> Option<&Value> {
    value.item(0)
}

fn same_type(lower: Option<&Value>, upper: &Value) -> bool {
    if matches!(lower, Some(Value::Array(items)) if items.is_empty()) {
        return true;
    }
    if matches!(upper, Value::Array(items) if items.is_empty()) {
        return true;
    }
    let Some(a) = lower.and_then(first) else { return false };
    let Some(b) = first(upper) else { return false };
    match (a, b) {
        (Value::Raw { type_key: a, .. }, Value::Raw { type_key: b, .. }) => a == b,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

pub(super) fn numbers(value: Option<&Value>) -> Vec<f32> {
    let Some(value) = value else { return Vec::new() };
    match value {
        Value::Array(items) => items.iter().flat_map(|v| numbers(Some(v))).collect(),
        Value::Raw { type_key, bytes } if *type_key == vlt_hash("AxlePair") && bytes.len() == 8 => {
            bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect()
        }
        _ => value.as_f32().or_else(|| value.as_i64().map(|v| v as f32)).into_iter().collect(),
    }
}

pub(super) fn blend(low: &[f32], high: &[f32], fraction: f32) -> Vec<f32> {
    (0..low.len().max(high.len()))
        .map(|i| low.get(i).copied().unwrap_or(0.0) * (1.0 - fraction) + high.get(i).copied().unwrap_or(0.0) * fraction)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_numeric_vectors_cover_unequal_arrays_and_axle_pairs() {
        assert_eq!(blend(&[10.0, 20.0], &[30.0, 60.0], 0.25), [15.0, 30.0]);
        assert_eq!(blend(&[2.0, 4.0], &[6.0, 8.0, 10.0], 0.5), [4.0, 6.0, 5.0]);
        assert_eq!(blend(&[10.0, 20.0, 30.0], &[30.0], 0.25), [15.0, 15.0, 22.5]);
        assert_eq!(blend(&[], &[10.0, 20.0], 0.25), [2.5, 5.0]);
        let pair = Value::Raw {
            type_key: vlt_hash("AxlePair"),
            bytes: [10.0f32, 20.0].into_iter().flat_map(f32::to_le_bytes).collect(),
        };
        assert_eq!(numbers(Some(&pair)), [10.0, 20.0]);
        assert_eq!(numbers(Some(&Value::Int32(-9))), [-9.0]);
        assert_eq!(blend(&[-9.0], &[8.0], 0.3)[0].trunc(), -3.0);
        assert!(same_type(Some(&Value::Array(vec![])), &Value::Array(vec![Value::Float(1.0)])));
    }
}
