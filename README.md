# database_Rust_basic

Rust言語で作成した、シンプルで堅牢なキーバリューストアです。

## 特徴

- `set/get/delete` をサポートする基本DB
- 変更履歴をログへ追記する永続化方式
- 再起動後もデータを復元
- `compact` によるログ圧縮

## 使い方

```bash
cargo run -- db.log set user alice
cargo run -- db.log get user
cargo run -- db.log list
cargo run -- db.log delete user
cargo run -- db.log compact
```

## テスト

```bash
cargo test
```
