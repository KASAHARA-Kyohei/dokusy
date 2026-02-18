# 2026-02-18 Zedシンタックスハイライト導入方針

## 背景
- `.dk` ファイルがZedでプレーンテキスト扱いされ、読みやすさが低かった。
- 言語本体はM0段階のため、まずは軽量にハイライトを提供したい。

## 判断内容
- `zed-extension/dokusy` に開発用拡張を追加する。
- grammar は当面 `tree-sitter-rust` を利用し、`.dk` の基本ハイライトを提供する。

## 代替案
- `.dk` を `Rust` に手動関連付けするだけの運用。
- `tree-sitter-dokusy` を先に新規実装してから導入する。

## 影響範囲
- 言語仕様（`docs/specs/m0.md`）には変更なし。
- エディタ体験のみ改善。

## 修正履歴
### 2026-02-18 (追記)
- `extension.toml` に `languages = ["languages/dokusy"]` を追加。
- grammar名を `dokusy_rust` から `rust` に変更。
  - 理由: `tree-sitter-rust` のエクスポート関数は `tree_sitter_rust` であり、`dokusy_rust` ではビルド時にシンボル不一致が発生したため。

### 2026-02-18 (追記2)
- `extension.toml` から `[grammars.rust]` を削除し、外部grammar取得を不要化。
  - 理由: イントラ制約で `tree-sitter-rust` のリビジョン取得に失敗し、dev extensionのインストールが失敗したため。

### 2026-02-18 (追記3)
- `highlights.scm` からキーワード/演算子の文字列マッチを一旦削除し、最小クエリ構成へ変更。
  - 理由: `Invalid node type "mut"` により言語ロード自体が失敗したため。

## フォローアップ
- `tree-sitter-dokusy` を作成後、`extension.toml` の grammar を差し替える。
- 必要になれば `outline.scm` や `brackets.scm` を追加する。
