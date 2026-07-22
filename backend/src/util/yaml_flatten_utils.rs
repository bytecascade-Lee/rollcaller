use anyhow::anyhow;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// 扁平化配置
#[derive(Debug, Clone)]
pub struct FlattenConfig {
    /// 键路径分隔符，默认 '.'
    pub separator: char,
    /// 是否保留空数组（使用特殊标记），默认 false
    pub preserve_empty_arrays: bool,
}

impl Default for FlattenConfig {
    fn default() -> Self {
        Self {
            separator: '.',
            preserve_empty_arrays: false,
        }
    }
}

/// 从文件读取并扁平化 YAML
pub fn flatten_from_file(
    path: &Path,
    config: &FlattenConfig
) -> anyhow::Result<HashMap<String, Value>> {
    let reader =
        BufReader::new(File::open(path).map_err(|e| anyhow!("无法打开文件 {:?}: {}", path, e))?);

    let value: Value =
        serde_yaml::from_reader(reader).map_err(|e| anyhow!("解析 YAML 文件失败: {}", e))?;

    flatten_from_value(&value, config)
}

/// 扁平化 YAML 值
pub fn flatten_from_value(
    value: &Value,
    config: &FlattenConfig,
) -> anyhow::Result<HashMap<String, Value>> {
    let mut result = HashMap::new();

    match value {
        Value::Mapping(map) => {
            for (key, val) in map {
                let key_str = key
                    .as_str()
                    .ok_or_else(|| anyhow!("Map 键必须是字符串类型"))?;
                flatten_node(key_str, val, config, &mut result)?;
            }
        }
        _ => {
            return Err(anyhow!("YAML 根必须是 Map 类型，当前类型: {:?}", value));
        }
    }

    Ok(result)
}

/// 递归扁平化节点
fn flatten_node(
    prefix: &str,
    value: &Value,
    config: &FlattenConfig,
    output: &mut HashMap<String, Value>,
) -> anyhow::Result<()> {
    match value {
        Value::Mapping(map) => {
            for (key, val) in map {
                let key_str = key
                    .as_str()
                    .ok_or_else(|| anyhow!("Map 键必须是字符串类型"))?;
                let full_key = format!("{}{}{}", prefix, config.separator, key_str);
                flatten_node(&full_key, val, config, output)?;
            }
        }

        Value::Sequence(seq) => {
            if seq.is_empty() {
                if config.preserve_empty_arrays {
                    // 使用特殊标记表示空数组
                    let marker = format!("{}__empty", prefix);
                    output.insert(marker, Value::Sequence(vec![]));
                }
                return Ok(());
            }

            for (idx, val) in seq.iter().enumerate() {
                let full_key = format!("{}{}[{}]", prefix, config.separator, idx);
                flatten_node(&full_key, val, config, output)?;
            }
        }

        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => {
            output.insert(prefix.to_string(), value.clone());
        }

        _ => {
            return Err(anyhow!("不支持的值类型: {:?} (键: {})", value, prefix));
        }
    }

    Ok(())
}

/// 将扁平化的键值对恢复为嵌套的 YAML 结构
pub fn unflatten_from_value(
    flat_map: &HashMap<String, Value>,
    config: &FlattenConfig,
) -> anyhow::Result<Value> {
    let mut root = Value::Mapping(serde_yaml::Mapping::new());

    // Sort by key length (shortest first) so parent paths are created before children,
    // making conflict detection deterministic regardless of HashMap iteration order.
    let mut entries: Vec<(&String, &Value)> = flat_map.iter().collect();
    entries.sort_by(|a, b| a.0.len().cmp(&b.0.len()));

    for (key, value) in entries {
        if config.preserve_empty_arrays && key.ends_with("__empty") {
            let real_key = key.trim_end_matches("__empty");
            let parts: Vec<String> = real_key
                .split(config.separator)
                .map(|s| s.to_string())
                .collect();
            if parts.is_empty() {
                return Err(anyhow!("键不能为空"));
            }
            insert_value(&mut root, &parts, Value::Sequence(vec![]), config)?;
            continue;
        }

        let parts: Vec<String> = key.split(config.separator).map(|s| s.to_string()).collect();
        if parts.is_empty() {
            return Err(anyhow!("键不能为空"));
        }

        // 处理数组索引格式 [0]
        let mut processed_parts = Vec::new();
        for part in &parts {
            if let Some(idx_str) = part.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                // 这是数组索引
                let idx: usize = idx_str
                    .parse()
                    .map_err(|_| anyhow!("无效的数组索引: {}", part))?;
                processed_parts.push(format!("[{}]", idx));
            } else {
                processed_parts.push(part.clone());
            }
        }

        insert_value(&mut root, &processed_parts, value.clone(), config)?;
    }

    Ok(root)
}

/// 递归插入值到嵌套结构
fn parse_array_index(part: &str) -> Option<usize> {
    part.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .and_then(|s| s.parse().ok())
}

fn next_container(next_part: &str) -> Value {
    if parse_array_index(next_part).is_some() {
        Value::Sequence(vec![])
    } else {
        Value::Mapping(serde_yaml::Mapping::new())
    }
}

/// 递归插入值到嵌套结构
fn insert_value(
    current: &mut Value,
    parts: &[String],
    value: Value,
    config: &FlattenConfig,
) -> anyhow::Result<()> {
    if parts.is_empty() {
        return Err(anyhow!("路径不能为空"));
    }

    let last_idx = parts.len() - 1;
    let sep = config.separator.to_string();
    let mut target = current;

    // Navigate through intermediate path segments, creating containers as needed
    for (i, part) in parts[..last_idx].iter().enumerate() {
        if let Some(idx) = parse_array_index(part) {
            // Array index segment — target should be a Sequence
            match target {
                Value::Sequence(seq) => {
                    while seq.len() <= idx {
                        let fill = next_container(&parts[i + 1]);
                        seq.push(fill);
                    }
                    target = &mut seq[idx];
                }
                _ => {
                    let path: String = parts[..=i].join(&sep);
                    return Err(anyhow!("路径 '{}' 不是数组类型", path));
                }
            }
        } else {
            // Map key segment — target should be a Mapping
            let key = Value::from(part.as_str());
            match target {
                Value::Mapping(map) => {
                    if !map.contains_key(&key) {
                        let fill = next_container(&parts[i + 1]);
                        map.insert(key.clone(), fill);
                    }
                    // Verify it's a container (releases shared borrow before get_mut)
                    if let Some(entry) = map.get(&key) {
                        if !entry.is_mapping() && !entry.is_sequence() {
                            let path: String = parts[..=i].join(&sep);
                            return Err(anyhow!("路径冲突: '{}' 不是容器类型，无法继续嵌套", path));
                        }
                    }
                    target = map.get_mut(&key).unwrap();
                }
                _ => {
                    let path: String = parts[..=i].join(&sep);
                    return Err(anyhow!("路径 '{}' 不是 Map 类型", path));
                }
            }
        }
    }
    let last = &parts[last_idx];
    if let Some(idx) = parse_array_index(last) {
        match target {
            Value::Sequence(seq) => {
                while seq.len() <= idx {
                    seq.push(Value::Null);
                }
                seq[idx] = value;
                Ok(())
            }
            _ => Err(anyhow!("路径 '{}' 不是数组类型", parts.join(&sep))),
        }
    } else {
        match target {
            Value::Mapping(map) => {
                map.insert(Value::from(last.as_str()), value);
                Ok(())
            }
            _ => Err(anyhow!("路径 '{}' 不是 Map 类型", parts.join(&sep))),
        }
    }
}

/// 从扁平化的键值对生成 YAML 字符串
pub fn unflatten_to_string(
    flat_map: &HashMap<String, Value>,
    config: &FlattenConfig,
) -> anyhow::Result<String> {
    let value = unflatten_from_value(flat_map, config)?;
    let yaml_string =
        serde_yaml::to_string(&value).map_err(|e| anyhow!("序列化为 YAML 失败: {}", e))?;
    Ok(yaml_string)
}

/// 使用默认配置从文件读取并扁平化 YAML
pub fn flatten_file_default(path: &Path) -> anyhow::Result<HashMap<String, Value>> {
    flatten_from_file(path, &FlattenConfig::default())
}

/// 使用默认配置扁平化 YAML 值
pub fn flatten_value_default(value: &Value) -> anyhow::Result<HashMap<String, Value>> {
    flatten_from_value(value, &FlattenConfig::default())
}

/// 使用默认配置反扁平化
pub fn unflatten_value_default(flat_map: &HashMap<String, Value>) -> anyhow::Result<Value> {
    unflatten_from_value(flat_map, &FlattenConfig::default())
}

/// 使用默认配置生成 YAML 字符串
pub fn unflatten_to_string_default(flat_map: &HashMap<String, Value>) -> anyhow::Result<String> {
    unflatten_to_string(flat_map, &FlattenConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use std::collections::HashMap;

    fn round_trip(yaml: &str) -> (HashMap<String, Value>, Value) {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        let unflat = unflatten_value_default(&flat).unwrap();
        (flat, unflat)
    }

    fn assert_round_trip(yaml: &str) {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let (_, unflat) = round_trip(yaml);
        assert_eq!(value, unflat, "round-trip failed for:\n{}", yaml);
    }

    fn assert_flatten(yaml: &str, expected: &[(&str, &str)]) {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        // Check count matches
        assert_eq!(
            flat.len(),
            expected.len(),
            "key count mismatch for:\n{}",
            yaml
        );
        for (key, yaml_val) in expected {
            let expected_val: Value = serde_yaml::from_str(yaml_val).unwrap();
            assert_eq!(
                flat.get(*key).unwrap(),
                &expected_val,
                "key '{}' mismatch in:\n{}",
                key,
                yaml
            );
        }
    }

    #[test]
    fn test_round_trip_simple_scalars() {
        assert_round_trip("a: hello\nb: 42\nc: true\nd: null\n");
    }

    #[test]
    fn test_round_trip_nested_mapping() {
        assert_round_trip("a:\n  b: c\n  d: e\n");
    }

    #[test]
    fn test_round_trip_three_levels() {
        assert_round_trip("a:\n  b:\n    c: deep\n");
    }

    #[test]
    fn test_round_trip_mixed_deep() {
        assert_round_trip(
            r#"
server:
  host: localhost
  port: 8080
  tls:
    enabled: true
    cert_path: /etc/cert.pem
database:
  name: mydb
  pool: 10
"#,
        );
    }

    #[test]
    fn test_round_trip_array_of_scalars() {
        assert_round_trip("items:\n  - a\n  - b\n  - c\n");
    }

    #[test]
    fn test_round_trip_array_of_mappings() {
        assert_round_trip("users:\n  - name: alice\n    age: 30\n  - name: bob\n    age: 25\n");
    }

    #[test]
    fn test_round_trip_nested_array() {
        assert_round_trip("matrix:\n  - [1, 2]\n  - [3, 4]\n");
    }

    #[test]
    fn test_round_trip_complex_nested() {
        assert_round_trip(
            r#"
api:
  versions:
    - v1
    - v2
  endpoints:
    - path: /users
      methods: [GET, POST]
    - path: /health
      methods: [GET]
  config:
    rate_limit: 100
    retry: true
"#,
        );
    }

    #[test]
    fn test_flatten_empty_mapping_and_array_dropped_by_default() {
        // Empty mappings and arrays produce no flat entries with default config.
        let yaml = "empty_map: {}\nempty_arr: []\nkeep: val\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat.get("keep").unwrap(), &Value::from("val"));
    }

    #[test]
    fn test_round_trip_float_and_negative() {
        assert_round_trip("pi: 3.14\ntemp: -5\n");
    }

    #[test]
    fn test_flatten_keys_with_separator_char() {
        // Keys containing the separator '.' are NOT round-trippable (expected limitation).
        // This test validates flatten output only.
        let yaml = "a.b: value\nx_y: 1\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        assert_eq!(flat.get("a.b").unwrap(), &Value::from("value"));
        assert_eq!(
            flat.get("x_y").unwrap(),
            &Value::Number(serde_yaml::Number::from(1))
        );
    }

    #[test]
    fn test_flatten_simple() {
        assert_flatten("a: 1\nb: 2\n", &[("a", "1"), ("b", "2")]);
    }

    #[test]
    fn test_flatten_nested_with_separator() {
        assert_flatten("a:\n  b: 1\n  c: 2\n", &[("a.b", "1"), ("a.c", "2")]);
    }

    #[test]
    fn test_flatten_array_indices() {
        assert_flatten(
            "items:\n  - x\n  - y\n",
            &[("items.[0]", "x"), ("items.[1]", "y")],
        );
    }

    #[test]
    fn test_flatten_deep_array_in_mapping() {
        assert_flatten(
            "a:\n  b:\n    - 1\n    - 2\n",
            &[("a.b.[0]", "1"), ("a.b.[1]", "2")],
        );
    }

    #[test]
    fn test_flatten_multiple_arrays() {
        assert_flatten(
            r#"
a:
  - 1
  - 2
b:
  - x
"#,
            &[("a.[0]", "1"), ("a.[1]", "2"), ("b.[0]", "x")],
        );
    }

    #[test]
    fn test_flatten_empty_array_no_marker() {
        let yaml = "tags: []\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        assert_eq!(
            flat.len(),
            0,
            "empty array should produce no entries with default config"
        );
    }

    #[test]
    fn test_flatten_empty_array_with_marker() {
        let yaml = "tags: []\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let config = FlattenConfig {
            preserve_empty_arrays: true,
            ..Default::default()
        };
        let flat = flatten_from_value(&value, &config).unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat.get("tags__empty").unwrap(), &Value::Sequence(vec![]));
    }

    #[test]
    fn test_flatten_empty_array_in_nested() {
        let yaml = "a:\n  b: []\n  c: 1\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let config = FlattenConfig {
            preserve_empty_arrays: true,
            ..Default::default()
        };
        let flat = flatten_from_value(&value, &config).unwrap();
        assert!(flat.contains_key("a.b__empty"));
        assert_eq!(
            flat.get("a.c").unwrap(),
            &Value::Number(serde_yaml::Number::from(1))
        );
    }

    #[test]
    fn test_unflatten_with_array_indices() {
        let mut flat = HashMap::new();
        flat.insert("items.[0]".to_string(), Value::from("a"));
        flat.insert("items.[1]".to_string(), Value::from("b"));
        let result = unflatten_value_default(&flat).unwrap();
        let expected: Value = serde_yaml::from_str("items:\n  - a\n  - b\n").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_unflatten_preserve_empty_array() {
        let mut flat = HashMap::new();
        flat.insert("tags__empty".to_string(), Value::Sequence(vec![]));
        let config = FlattenConfig {
            preserve_empty_arrays: true,
            ..Default::default()
        };
        let result = unflatten_from_value(&flat, &config).unwrap();
        let expected: Value = serde_yaml::from_str("tags: []\n").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_unflatten_conflict_non_map_intermediate() {
        let mut flat = HashMap::new();
        flat.insert("a.b".to_string(), Value::from("leaf"));
        flat.insert("a.b.c".to_string(), Value::from("nested"));
        let result = unflatten_value_default(&flat);
        assert!(
            result.is_err(),
            "should error when 'a.b' is leaf but also has child 'a.b.c'"
        );
    }

    #[test]
    fn test_custom_separator_round_trip() {
        let yaml = "a:\n  b: 1\n  c: 2\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let config = FlattenConfig {
            separator: '/',
            ..Default::default()
        };
        let flat = flatten_from_value(&value, &config).unwrap();
        assert!(flat.contains_key("a/b"));
        assert!(flat.contains_key("a/c"));
        let unflat = unflatten_from_value(&flat, &config).unwrap();
        assert_eq!(value, unflat);
    }

    #[test]
    fn test_custom_separator_preserve_empty() {
        let yaml = "a: []\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let config = FlattenConfig {
            separator: '/',
            preserve_empty_arrays: true,
        };
        let flat = flatten_from_value(&value, &config).unwrap();
        assert!(flat.contains_key("a__empty"));
        let unflat = unflatten_from_value(&flat, &config).unwrap();
        assert_eq!(value, unflat);
    }

    #[test]
    fn test_flatten_non_map_root() {
        let value: Value = serde_yaml::from_str("[1, 2, 3]").unwrap();
        let result = flatten_value_default(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("根必须是 Map"));
    }

    #[test]
    fn test_flatten_scalar_root() {
        let value: Value = serde_yaml::from_str("hello").unwrap();
        let result = flatten_value_default(&value);
        assert!(result.is_err());
    }

    #[test]
    fn test_unflatten_empty_flat_map() {
        let flat = HashMap::new();
        let result = unflatten_value_default(&flat).unwrap();
        assert!(result.is_mapping());
        assert!(result.as_mapping().unwrap().is_empty());
    }

    #[test]
    fn test_preserve_empty_array_without_config_is_noop() {
        let mut flat = HashMap::new();
        flat.insert("tags__empty".to_string(), Value::Sequence(vec![]));
        // default config: preserve_empty_arrays = false
        let result = unflatten_value_default(&flat).unwrap();
        // "__empty" suffix treated as a literal key part
        let expected: Value = serde_yaml::from_str("tags__empty: []\n").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_unflatten_to_string_round_trip() {
        let yaml = "a:\n  b: hello\n  c: [1, 2]\n";
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let flat = flatten_value_default(&value).unwrap();
        let output = unflatten_to_string_default(&flat).unwrap();
        let reparsed: Value = serde_yaml::from_str(&output).unwrap();
        assert_eq!(value, reparsed);
    }

    #[test]
    fn test_real_world_config() {
        let yaml = r#"
app:
  name: my-service
  version: 2.1.0
  debug: false
logging:
  level: info
  outputs:
    - type: file
      path: /var/log/app.log
    - type: stdout
  metrics:
    enabled: true
    tags:
      env: production
      region: us-east-1
noop: null
"#;
        assert_round_trip(yaml);
    }
}
