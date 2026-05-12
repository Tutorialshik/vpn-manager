#!/usr/bin/env bash
set -euo pipefail

echo "=== 1. Форматирование (cargo fmt) ==="
cargo fmt --check

echo "=== 2. Статический анализ (cargo clippy) ==="
cargo clippy -- -D warnings

echo "=== 3. Тесты (cargo test) ==="
cargo test

echo "=== 4. Аудит зависимостей (cargo audit) ==="
if ! command -v cargo-audit &> /dev/null; then
    echo "Устанавливаю cargo-audit..."
    cargo install cargo-audit
fi
cargo audit

echo "=== Все проверки пройдены успешно! ==="
