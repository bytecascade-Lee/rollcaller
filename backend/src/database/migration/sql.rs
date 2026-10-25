//! 编译进二进制的迁移脚本

pub(crate) struct SqlMigration {
    pub version: i64,
    pub name: &'static str,
    /// `include_str!` 的原始内容
    pub sql: &'static str,
}

pub(crate) const MIGRATIONS: &[SqlMigration] = &[
    SqlMigration {
        version: 0,
        name: "initialize schema",
        sql: include_str!("../../../../resources/database/migrations/V0__initialize-schema.sql"),
    },
    SqlMigration {
        version: 20,
        name: "create attendance_status_definition table",
        sql: include_str!("../../../../resources/database/migrations/V20__create-attendance_status_definition-table.sql"),
    },
    SqlMigration {
        version: 30,
        name: "add foreign key to attendance_status",
        sql: include_str!("../../../../resources/database/migrations/V30__add-foreign-key-to-attendance_status.sql"),
    },
];

/// 规范化：去 BOM、CRLF -> LF、去掉文件末尾的空行
///
/// 只动"无意义的字符"：换行符与文件末尾空行。
/// **不要**逐行 trim 尾随空格——那可能改到 SQL 字符串字面量里的内容。
pub(crate) fn normalize(raw: &str) -> String {
    let body = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    body.replace("\r\n", "\n").trim_end_matches('\n').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ext::hash_ext::HashExt;

    /// 已发布迁移的规范化文本哈希
    const FROZEN: &[(i64, &str)] = &[
        (0, "e8604196400db71af3f171bda2dd96db44b6459617b6ef230502e4470710c7c6"),
        (20, "177aaf77820dbb74f48bcd5b8d63b54083352a02079244a800a4926e00760ce1"),
        (30, "cdd1e6e2fe4ccc78e258ed456e3004171f5d1ebf41dcc5c599dec084cda63e93"),
    ];

    #[test]
    fn versions_are_strictly_increasing() {
        for pair in MIGRATIONS.windows(2) {
            assert!(
                pair[0].version < pair[1].version,
                "迁移版本必须严格递增: {} -> {}",
                pair[0].version,
                pair[1].version
            );
        }
    }

    #[test]
    fn published_migrations_are_frozen() {
        assert_eq!(MIGRATIONS.len(), FROZEN.len(), "迁移数量变了，请同步 FROZEN");
        for (version, expected) in FROZEN {
            let migration = MIGRATIONS
                .iter()
                .find(|m| m.version == *version)
                .unwrap_or_else(|| panic!("缺少 V{version}"));
            assert_eq!(
                &normalize(migration.sql).sha256(),
                *expected,
                "V{version}={} 的内容已被改动；已发布的迁移不允许修改，请新增一条",
                migration.name
            );
        }
    }

    #[test]
    fn normalize_strips_eol_noise() {
        assert_eq!(normalize("SELECT 1;\n\n\n"), "SELECT 1;");
        assert_eq!(normalize("SELECT 1;\r\n"), "SELECT 1;");
        assert_eq!(normalize("\u{feff}SELECT 1;\r\nSELECT 2;\r\n"), "SELECT 1;\nSELECT 2;");
        assert_eq!(normalize("SELECT 'a  ';\n"), "SELECT 'a  ';");
    }
}
