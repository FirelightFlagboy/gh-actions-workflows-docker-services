use std::collections::HashMap;

pub fn ordered_map<S, K, V>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    K: Ord + serde::Serialize,
    V: serde::Serialize,
{
    use itertools::Itertools;

    serializer.collect_map(value.iter().sorted_by_key(|(k, _v)| *k))
}
