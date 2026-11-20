//! JSON output (APIs, `kubectl -o json`, `gh api`): minify, turn arrays of
//! same-shaped objects into column/row form, and sample very long arrays.
//! The result is still valid JSON.

use serde_json::{Map, Value};

const MIN_TABLE_ROWS: usize = 5;
const MAX_ROWS: usize = 40;
const HEAD: usize = 25;
const TAIL: usize = 10;
const MAX_ERROR_ROWS: usize = 10;
const MAX_STR: usize = 1500;

pub fn parse(raw: &str) -> Option<Value> {
    let t = raw.trim();
    if t.len() < 2 || !(t.starts_with('{') || t.starts_with('[')) {
        return None;
    }
    serde_json::from_str(t).ok()
}

pub fn compress(v: Value) -> String {
    serde_json::to_string(&shrink(v)).unwrap_or_default()
}

fn shrink(v: Value) -> Value {
    match v {
        Value::Array(items) => shrink_array(items),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, shrink(v))).collect()),
        Value::String(s) if s.chars().count() > MAX_STR => {
            let n = s.chars().count();
            let head: String = s.chars().take(MAX_STR).collect();
            Value::String(format!("{head}…[+{} chars]", n - MAX_STR))
        }
        other => other,
    }
}

fn shrink_array(items: Vec<Value>) -> Value {
    let items: Vec<Value> = items.into_iter().map(shrink).collect();
    let n = items.len();
    if n >= MIN_TABLE_ROWS && items.iter().all(Value::is_object) {
        let objs: Vec<&Map<String, Value>> = items.iter().filter_map(Value::as_object).collect();
        let mut cols: Vec<String> = Vec::new();
        for o in &objs {
            for k in o.keys() {
                if !cols.contains(k) {
                    cols.push(k.clone());
                }
            }
        }
        let avg = objs.iter().map(|o| o.len()).sum::<usize>() as f64 / n as f64;
        if !cols.is_empty() && cols.len() <= 40 && (cols.len() as f64) <= avg * 1.5 + 1.0 {
            let rows: Vec<Value> = objs
                .iter()
                .map(|o| {
                    Value::Array(
                        cols.iter()
                            .map(|c| o.get(c).cloned().unwrap_or(Value::Null))
                            .collect(),
                    )
                })
                .collect();
            let (rows, omitted) = sample(rows);
            let mut m = Map::new();
            m.insert("_cols".into(), Value::from(cols));
            m.insert("_rows".into(), Value::Array(rows));
            if omitted > 0 {
                m.insert("_omitted_rows".into(), Value::from(omitted));
            }
            return Value::Object(m);
        }
    }
    let (mut items, omitted) = sample(items);
    if omitted > 0 {
        items.insert(HEAD, Value::String(format!("…[{omitted} items omitted]")));
    }
    Value::Array(items)
}

fn looks_errorish(v: &Value) -> bool {
    let s = v.to_string().to_ascii_lowercase();
    s.contains("error") || s.contains("fail") || s.contains("exception")
}

/// Keep the first HEAD and last TAIL items plus up to MAX_ERROR_ROWS error-looking ones.
fn sample(items: Vec<Value>) -> (Vec<Value>, usize) {
    let n = items.len();
    if n <= MAX_ROWS {
        return (items, 0);
    }
    let mut keep = vec![false; n];
    keep[..HEAD].iter_mut().for_each(|k| *k = true);
    keep[n - TAIL..].iter_mut().for_each(|k| *k = true);
    let mut extra = 0;
    for i in HEAD..n - TAIL {
        if extra >= MAX_ERROR_ROWS {
            break;
        }
        if looks_errorish(&items[i]) {
            keep[i] = true;
            extra += 1;
        }
    }
    let omitted = keep.iter().filter(|k| !**k).count();
    let kept = items
        .into_iter()
        .zip(keep)
        .filter_map(|(v, k)| k.then_some(v))
        .collect();
    (kept, omitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabulates_uniform_objects() {
        let raw = r#"[{"id":1,"name":"a"},{"id":2,"name":"b"},{"id":3,"name":"c"},{"id":4,"name":"d"},{"id":5,"name":"e"}]"#;
        let out = compress(parse(raw).unwrap());
        assert_eq!(
            out,
            r#"{"_cols":["id","name"],"_rows":[[1,"a"],[2,"b"],[3,"c"],[4,"d"],[5,"e"]]}"#
        );
    }

    #[test]
    fn samples_long_arrays_keeping_errors() {
        let items: Vec<String> = (0..100)
            .map(|i| {
                let status = if i == 60 { "error" } else { "ok" };
                format!(r#"{{"i":{i},"status":"{status}"}}"#)
            })
            .collect();
        let raw = format!("[{}]", items.join(","));
        let out = compress(parse(&raw).unwrap());
        assert!(out.contains(r#"[60,"error"]"#));
        assert!(out.contains(r#""_omitted_rows":64"#));
    }

    #[test]
    fn non_json_is_ignored() {
        assert!(parse("[INFO] building").is_none());
    }
}
