# xterm.js + Claude Code 互換性向上計画

## 概要

本ドキュメントは、Kiriのターミナル（xterm.js）でClaude Codeを使用する際に発生する問題を調査し、今後の改善計画をまとめたものです。

## 調査結果サマリー

### 現在のKiriの実装状況

Kiriのターミナル実装は既にClaude Code対応を意識した設計になっています：

| 機能 | 実装状況 | 備考 |
|------|----------|------|
| MAX_TERMINAL_COLS = 120 | ✅ 実装済 | Inkスピナーバグ対策 |
| Synchronized Output (Mode 2026) | ✅ 実装済 | フレームバッファリング |
| 正確なPTYサイズ管理 | ✅ 実装済 | レイアウト完了待機 |
| TERM=xterm-256color | ✅ 実装済 | ANSIシーケンス対応 |
| scrollOnEraseInDisplay | ✅ 実装済 | ED2対応 |

### 既知の問題（GitHub Issues調査）

#### 1. フリッカリング問題（高優先度）
**参照**: [claude-code#1913](https://github.com/anthropics/claude-code/issues/1913) (252👍), [claude-code#769](https://github.com/anthropics/claude-code/issues/769)

**原因**:
- Inkフレームワークがステータス更新時に全バッファを再描画
- ターミナル高さと完全一致する要素のレンダリング時にスクロール発生
- 大きな出力や長いセッションで悪化

**現状の対策**:
- Synchronized Output Mode 2026は既に実装済み
- MAX_TERMINAL_COLSによる列数制限も実装済み

**追加対策候補**:
- ターミナル高さの1行余裕を持たせる
- Windows Terminal使用時の`enableUnfocusedAcrylic: false`設定の案内

#### 2. プログレスバー/スピナーの表示問題
**参照**: [claude-code#13637](https://github.com/anthropics/claude-code/issues/13637)

**原因**:
- キャリッジリターン（`\r`）が正しく解釈されない場合がある
- 中間状態が全て連結表示される

**現状**:
- xterm.jsは`\r`を正しく処理するため、Kiriでは問題発生しにくい
- Synchronized Output実装により部分フレーム表示を抑制

#### 3. ANSIエスケープシーケンスの互換性
**参照**: [xterm.js#1204](https://github.com/xtermjs/xterm.js/issues/1204), [xterm.js#1395](https://github.com/xtermjs/xterm.js/issues/1395)

**原因**:
- 一部のC0制御コードが未対応
- DEC/ANSI準拠の完全性に課題

**影響度**: 低（Claude Codeでは標準的なシーケンスのみ使用）

#### 4. 通知サポート
**参照**: [claude-code#7239](https://github.com/anthropics/claude-code/issues/7239)

**要望**:
- タスク完了時の通知メカニズム
- xterm.jsベースのターミナルでの通知対応

---

## 改善計画

### Phase 1: 既存実装の検証と微調整（短期）

#### 1.1 フリッカリング軽減の追加対策

**現状確認が必要な項目**:
- [ ] Synchronized Output Modeが正しく動作しているか確認
- [ ] フレームバッファリングのタイミング調整

**追加実装候補**:
```typescript
// ターミナル高さに余裕を持たせる（Ink Issue #450対策）
const effectiveRows = Math.max(terminal.rows - 1, 10);
```

#### 1.2 リサイズ時の安定性向上

**現状**:
- ゼロサイズ保護は実装済み
- Debounce処理で頻繁なリサイズを抑制

**追加検討**:
- [ ] リサイズ中のSynchronized Output強制有効化
- [ ] リサイズ完了後の安定待機時間追加

### Phase 2: 互換性の強化（中期）

#### 2.1 Alternate Screen Buffer対応の強化

Claude Codeのようなフルスクリーンアプリケーションは通常、Alternate Screen Buffer（DECSET 1049）を使用します。

**確認項目**:
- [ ] Alternate Screenへの切り替えが正しく動作するか
- [ ] 終了時のメインスクリーンへの復帰が正しいか

#### 2.2 Cursor Position Save/Restore

**対応シーケンス**:
- DECSC (`ESC 7`) / DECRC (`ESC 8`)
- CSI s / CSI u

#### 2.3 OSC Hyperlinks

Claude Codeが出力するファイルパスリンクの検出を強化：

**現状**: `filePathLinkProvider.ts`で正規表現ベースのリンク検出

**改善候補**:
- OSC 8形式のハイパーリンク対応
- `file://` URLスキームの処理

### Phase 3: ユーザー体験の向上（長期）

#### 3.1 通知システムの実装

**実装案**:
```typescript
// OSC 9 (iTerm2 notification) または OSC 777 (rxvt-unicode) 検出
const OSC_NOTIFICATION_PATTERNS = [
  /\x1b\]9;(.+?)\x07/,  // iTerm2
  /\x1b\]777;notify;(.+?);(.+?)\x07/  // rxvt
];

// Tauriのシステム通知APIと連携
```

**機能**:
- タスク完了時のデスクトップ通知
- サウンド通知オプション
- フォーカス時の通知抑制

#### 3.2 ターミナル設定のプリセット

Claude Code最適化プリセット：
```typescript
const CLAUDE_CODE_PRESET = {
  maxCols: 120,
  syncOutputEnabled: true,
  scrollback: 10000,
  fontFamily: "'JetBrains Mono', monospace",
  fontSize: 13,
};
```

#### 3.3 問題診断ツール

ターミナル互換性チェック機能：
- [ ] Synchronized Output対応確認
- [ ] ANSIシーケンス対応確認
- [ ] 現在のターミナルサイズ表示
- [ ] 推奨設定の提示

---

## 技術的詳細

### Synchronized Output Mode (DEC Private Mode 2026)

**仕様**:
- `\x1b[?2026h` - 同期モード開始（出力をバッファリング）
- `\x1b[?2026l` - 同期モード終了（バッファをフラッシュ）

**現在の実装** (`Terminal.svelte`):
```typescript
const SYNC_START = '\x1b[?2026h';
const SYNC_END = '\x1b[?2026l';

// xterm.jsはMode 2026をネイティブサポートしていないため
// requestAnimationFrameでバッチ書き込み
```

**参考**: [Synchronized Output Spec](https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036)

### Inkフレームワークの制限

Inkは以下の制限があります：
- 高さがターミナル行数と完全一致すると意図しないスクロール発生
- 頻繁な更新時に全バッファ再描画
- 長いセッションでパフォーマンス低下

**Kiriでの対策**:
- MAX_TERMINAL_COLS = 120で水平方向のグリッチを防止
- Synchronized Outputで部分フレーム表示を抑制

---

## 参考リソース

### GitHub Issues
- [claude-code#1913: Terminal Flickering](https://github.com/anthropics/claude-code/issues/1913) (252👍)
- [claude-code#769: Screen Flickering](https://github.com/anthropics/claude-code/issues/769)
- [claude-code#13637: Progress bar handling](https://github.com/anthropics/claude-code/issues/13637)
- [ink#359: Flickering when view exceeds screen](https://github.com/vadimdemedes/ink/issues/359)
- [ink#450: Flickering at full height](https://github.com/vadimdemedes/ink/issues/450)

### 仕様・ドキュメント
- [Synchronized Output Spec](https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036)
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ANSI Escape Code Standards](https://jvns.ca/blog/2025/03/07/escape-code-standards/)

---

## 実装状況

### Phase 1: 完了 ✅

| タスク | 状況 | コミット |
|--------|------|----------|
| Synchronized Output Mode確認 | ✅ 完了 | b47bf72 |
| デバッグログ追加 | ✅ 完了 | b47bf72 |
| PTY_ROW_MARGIN (rows - 1) | ✅ 完了 | b47bf72 |
| リサイズバッファリング | ✅ 完了 | 9ad8662 |
| 安定待機時間 (50ms) | ✅ 完了 | 9ad8662 |

### Phase 2: 未着手

- Alternate Screen Buffer対応確認
- Cursor Position Save/Restore
- OSC Hyperlinks

### Phase 3: 未着手

- 通知システム
- ターミナル設定プリセット
- 問題診断ツール

---

## 次のアクション

1. **Phase 2**: Alternate Screen Buffer対応の確認
1. **Phase 2**: OSC 8形式ハイパーリンク対応
1. **Phase 3**: 通知システムの設計と実装
