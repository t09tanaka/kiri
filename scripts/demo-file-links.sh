#!/bin/bash

# Demo script for testing file path links in Kiri terminal
# Run this script in the Kiri terminal to see clickable file paths

echo ""
echo "============================================"
echo "  Kiri File Link Demo"
echo "  ファイルパスリンクのデモ"
echo "============================================"
echo ""

echo "📁 Basic file paths (クリックでプレビュー):"
echo "   src/App.svelte"
echo "   src/lib/stores/tabStore.ts"
echo "   src/lib/components/terminal/Terminal.svelte"
echo ""

echo "📍 Paths with line numbers:"
echo "   src/lib/services/filePathLinkProvider.ts:25"
echo "   src/lib/stores/peekStore.ts:29"
echo "   src/lib/components/peek/PeekEditor.svelte:168"
echo ""

echo "📌 Paths with line and column:"
echo "   src/App.svelte:10:5"
echo "   src/lib/utils/date.ts:42:12"
echo "   src-tauri/src/main.rs:1:1"
echo ""

echo "🔗 Relative paths:"
echo "   ./package.json"
echo "   ./tsconfig.json"
echo "   ./src/main.ts"
echo ""

echo "📝 Config files:"
echo "   vite.config.ts"
echo "   tailwind.config.js"
echo "   CLAUDE.md"
echo ""

echo "🦀 Rust files:"
echo "   src-tauri/src/lib.rs"
echo "   src-tauri/src/commands/terminal.rs:50"
echo "   src-tauri/Cargo.toml"
echo ""

echo "📘 TypeScript files (Stores):"
echo "   src/lib/stores/tabStore.ts:1"
echo "   src/lib/stores/projectStore.ts:25"
echo "   src/lib/stores/peekStore.ts:29"
echo "   src/lib/stores/gitStore.ts:10"
echo "   src/lib/stores/appStore.ts:1"
echo "   src/lib/stores/toastStore.ts:15"
echo "   src/lib/stores/searchStore.ts:1"
echo "   src/lib/stores/terminalRegistry.ts:5"
echo ""

echo "📗 TypeScript files (Services):"
echo "   src/lib/services/filePathLinkProvider.ts:25"
echo "   src/lib/services/suggestService.ts:1"
echo "   src/lib/services/persistenceService.ts:10"
echo ""

echo "📙 TypeScript files (Utils & Types):"
echo "   src/main.ts:1"
echo "   src/lib/utils/fileIcons.ts:50"
echo "   src/lib/components/filetree/types.ts:1"
echo "   src/lib/components/editor/languages.ts:20"
echo ""

echo "============================================"
echo "  Cmd/Ctrl+クリック でファイルをプレビュー"
echo "  ホバーでハイライト表示"
echo "============================================"
echo ""
