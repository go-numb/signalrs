# ビルドガイド

## 環境別ビルド方法

### Windows（ネイティブ）

最も簡単な方法。

```powershell
cd D:\Documents\@go-numb\signalrs\signal-clicker
npm install
npm run tauri build
```

**前提条件:**
- Visual Studio Build Tools（C++ ワークロード）
- Node.js
- Rust（`rustup` でインストール）

---

### WSL / Linux → Windows クロスコンパイル

WSL (Ubuntu 24.04) から Windows 向け `.exe` を生成する。

#### 1. 必要ツールのインストール

```bash
# Rust Windows ターゲット
rustup target add x86_64-pc-windows-msvc

# cargo-xwin（MSVC SDK を自動ダウンロード）
cargo install cargo-xwin

# クロスコンパイルに必要なツール
sudo apt-get install -y clang lld llvm

# clang-cl シンボリックリンク（なければ作成）
sudo ln -s /usr/bin/clang /usr/bin/clang-cl
```

#### 2. フロントエンドビルド

```bash
cd /path/to/signal-clicker
npm install
npm run build
```

#### 3. Windows 向け exe 生成

```bash
cd /path/to/signal-clicker/src-tauri
TAURI_CONFIG='{"build":{"devUrl":null}}' \
  cargo xwin build --release --target x86_64-pc-windows-msvc
```

**重要:** `TAURI_CONFIG='{"build":{"devUrl":null}}'` を付けないと、dev モードとしてビルドされ、`localhost:1420` に接続しようとして起動に失敗する。

生成物:
```
src-tauri/target/x86_64-pc-windows-msvc/release/sig-clicker.exe
```

#### 4. VPN に関する注意

`cargo-xwin` は初回ビルド時に `https://aka.ms/vs/17/release/channel` から MSVC SDK をダウンロードする。VPN が有効だと DNS 解決に失敗する場合がある。初回ビルド時は VPN を無効化すること。

ダウンロード済み SDK はキャッシュされる:
```
~/.cache/cargo-xwin/xwin/
```

---

### WSL / Linux ネイティブビルド（開発用）

Linux 上で `tauri dev` を実行して開発する場合。

```bash
# Tauri v2 の Linux 依存ライブラリ（Ubuntu 24.04）
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

cd /path/to/signal-clicker
npm install
npm run tauri dev
```

**注意:** Tauri v2 は `webkit2gtk-4.1` を使用する。Tauri v1 の `webkit2gtk-4.0` は Ubuntu 24.04 では利用不可。

---

## トラブルシューティング

### `ERR_CONNECTION_REFUSED` (localhost) で起動失敗

フロントエンドが埋め込まれていない。`TAURI_CONFIG='{"build":{"devUrl":null}}'` を付けてビルドし直す。

### ポート 8080 が使用中

```powershell
# Windows
netstat -ano | findstr :8080
taskkill /PID <PID> /F
```

アプリは TCP bind 失敗時もGUIは起動する（エラーログのみ出力）。

### `clang-cl` が見つからない

```bash
sudo ln -s /usr/bin/clang /usr/bin/clang-cl
```

### パニックでウィンドウが一瞬で消える

デバッグ用にコンソールを有効化してビルド:

`main.rs` の `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` をコメントアウトしてリビルド。

### 有効期限エラー

`main.rs` の `set_jst_expired` の日付が過去になっていないか確認。
