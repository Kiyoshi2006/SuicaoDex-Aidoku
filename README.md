# SuicaoDex for Aidoku

Extension Aidoku để đọc manga từ [SuicaoDex](https://suicaodex.com) trên iPhone/iOS.

## Tải về

[![Build Status](https://github.com/Kiyoshi2006/SuicaoDex-Aidoku/actions/workflows/build.yml/badge.svg)](https://github.com/Kiyoshi2006/SuicaoDex-Aidoku/actions/workflows/build.yml)

→ **[Tải SuicaoDex.aix từ Releases](../../releases/latest)**

## Cài đặt

1. Tải file `SuicaoDex.aix` từ [Releases](../../releases/latest)
2. Mở **Aidoku** trên iPhone
3. **Settings → Sources → +** → chọn file `.aix`

## Tính năng

- 🔍 Tìm kiếm truyện
- 🆕 Danh sách mới cập nhật (Latest)
- 📖 Chi tiết truyện: bìa, mô tả, tác giả, thể loại, trạng thái
- 📋 Danh sách chương đầy đủ (nhóm dịch, ngôn ngữ, ngày đăng)
- 🖼️ Đọc ảnh với fallback: optimizedUrl → replicaUrl → url gốc
- 📦 Tải offline qua Aidoku Reader
- 📄 Phân trang

## Build tự tay

Yêu cầu: Rust stable, `wasm32-unknown-unknown` target, aidoku CLI.

```bash
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-cli
cargo build --release --target wasm32-unknown-unknown
aidoku package
aidoku verify *.aix
```

## API

Extension kết nối trực tiếp với `https://api.suicaodex.com` — không cần VPS, không cần proxy.

## License

MIT
