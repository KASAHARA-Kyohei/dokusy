# dokusy (M0)

Rustで実装した小さな自作言語 `dokusy` のM0インタプリタです。

## 特徴

- `let` / `let mut` による変数宣言
- `if` / `while` / ブロック `{ ... }`
- 関数定義と関数呼び出し
- 組み込み `print(expr);`
- i64 / bool / string
- `&` と `::` は非対応（字句エラー）

## ビルド

```bash
cargo build
```

## 実行

```bash
cargo run -- run examples/hello.dk
cargo run -- run examples/while_sum.dk
cargo run -- run examples/functions.dk
```

## REPL

```bash
cargo run -- repl
```

M0ではプレースホルダ（未実装表示のみ）です。

## テスト

```bash
cargo test
```

- lexer / parser / interpreter の単体テストを含みます。

## CLI

- `dokusy run <file.dk>`
- `dokusy repl`

## Zed シンタックスハイライト（開発用拡張）

`zed-extension/dokusy` に `.dk` 用の開発拡張を同梱しています。

```bash
open zed-extension/dokusy
```

Zed で Command Palette から `zed: install dev extension` を実行し、上記ディレクトリを指定してください。
