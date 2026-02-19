use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    CorruptLog(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Io(err) => write!(f, "I/O error: {err}"),
            DbError::CorruptLog(line) => write!(f, "Corrupt log line: {line}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<std::io::Error> for DbError {
    fn from(value: std::io::Error) -> Self {
        DbError::Io(value)
    }
}

pub struct Database {
    data: BTreeMap<String, String>,
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            File::create(&path)?;
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut data = BTreeMap::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            apply_log_line(&mut data, &line)?;
        }

        Ok(Self { data, path })
    }

    pub fn set(&mut self, key: String, value: String) -> Result<(), DbError> {
        self.data.insert(key.clone(), value.clone());
        self.append_line(&format!("S\t{}\t{}", encode(&key), encode(&value)))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }

    pub fn delete(&mut self, key: &str) -> Result<bool, DbError> {
        let existed = self.data.remove(key).is_some();
        if existed {
            self.append_line(&format!("D\t{}", encode(key)))?;
        }
        Ok(existed)
    }

    pub fn list(&self) -> impl Iterator<Item = (&str, &str)> {
        self.data.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn compact(&self) -> Result<(), DbError> {
        let tmp_path = self.path.with_extension("tmp");
        let tmp_file = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(tmp_file);

        for (key, value) in &self.data {
            writeln!(writer, "S\t{}\t{}", encode(key), encode(value))?;
        }
        writer.flush()?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }

    fn append_line(&self, line: &str) -> Result<(), DbError> {
        let file = OpenOptions::new().append(true).open(&self.path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{line}")?;
        writer.flush()?;
        Ok(())
    }
}

fn apply_log_line(data: &mut BTreeMap<String, String>, line: &str) -> Result<(), DbError> {
    let mut parts = line.split('\t');
    match parts.next() {
        Some("S") => {
            let key = parts
                .next()
                .map(decode)
                .ok_or_else(|| DbError::CorruptLog(line.to_string()))?;
            let value = parts
                .next()
                .map(decode)
                .ok_or_else(|| DbError::CorruptLog(line.to_string()))?;
            if parts.next().is_some() {
                return Err(DbError::CorruptLog(line.to_string()));
            }
            data.insert(key, value);
            Ok(())
        }
        Some("D") => {
            let key = parts
                .next()
                .map(decode)
                .ok_or_else(|| DbError::CorruptLog(line.to_string()))?;
            if parts.next().is_some() {
                return Err(DbError::CorruptLog(line.to_string()));
            }
            data.remove(&key);
            Ok(())
        }
        _ => Err(DbError::CorruptLog(line.to_string())),
    }
}

fn encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

fn decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::Database;
    use std::path::PathBuf;

    fn tmp_file(name: &str) -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{ts}.log"))
    }

    #[test]
    fn set_get_delete_roundtrip() {
        let db_path = tmp_file("roundtrip");
        let mut db = Database::open(&db_path).expect("open db");

        db.set("name".to_string(), "codex".to_string())
            .expect("set");
        assert_eq!(db.get("name"), Some("codex"));
        assert!(db.delete("name").expect("delete"));
        assert_eq!(db.get("name"), None);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn data_survives_reopen() {
        let db_path = tmp_file("reopen");

        {
            let mut db = Database::open(&db_path).expect("open db");
            db.set("lang".to_string(), "rust".to_string()).expect("set");
        }

        let db = Database::open(&db_path).expect("reopen db");
        assert_eq!(db.get("lang"), Some("rust"));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn compact_rewrites_log() {
        let db_path = tmp_file("compact");
        let mut db = Database::open(&db_path).expect("open db");

        db.set("a".to_string(), "1".to_string()).expect("set a");
        db.set("b".to_string(), "2".to_string()).expect("set b");
        db.delete("a").expect("delete a");
        db.compact().expect("compact");

        let reloaded = Database::open(&db_path).expect("reload");
        assert_eq!(reloaded.get("a"), None);
        assert_eq!(reloaded.get("b"), Some("2"));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn escaped_chars_are_preserved() {
        let db_path = tmp_file("escaped");
        let mut db = Database::open(&db_path).expect("open db");

        db.set("line\tbreak".to_string(), "hello\nworld".to_string())
            .expect("set escaped");

        let reloaded = Database::open(&db_path).expect("reload");
        assert_eq!(reloaded.get("line\tbreak"), Some("hello\nworld"));

        let _ = std::fs::remove_file(db_path);
    }
}
