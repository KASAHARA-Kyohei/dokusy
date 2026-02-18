# Dokusy Zed Extension (Dev)

`dokusy` (`.dk`) 向けのシンタックスハイライト拡張です。

## インストール（開発用）

1. Zed を開く
2. `Cmd-Shift-P` で Command Palette を開く
3. `zed: install dev extension` を実行
4. このディレクトリを選択する
   - `/Users/kasaharakyouhei/dev/dokusy/zed-extension/dokusy`
5. `.dk` ファイルを開き、右下の言語モードが `Dokusy` になっていることを確認する

## 補足

- 現在はZed内蔵の `rust` grammar を利用します（外部grammarのfetch不要）。
- 初回導入時に反映されない場合は、再度 `zed: install dev extension` を実行してください。
- エラー確認は `~/Library/Logs/Zed/Zed.log` を参照してください。
- `dokusy` 専用 grammar を導入したら `extension.toml` の grammar を差し替えてください。
