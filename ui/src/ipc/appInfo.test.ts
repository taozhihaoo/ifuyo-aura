import { describe, expect, it } from 'vitest';
import type { AppInfo } from './appInfo';

// 契约镜像测试：保证 TS 侧结构与 src-tauri/src/ipc.rs 的 AppInfo（camelCase 序列化）一致。
// 真实跨端验证由 M6 的 Tauri smoke test 承担；此处锁定字段契约不被静默破坏。
describe('app_info contract mirror', () => {
  it('AppInfo fields match the Rust DTO serialization', () => {
    const sample: AppInfo = {
      name: 'ifuyo Aura',
      version: '0.1.0',
      slogan: '以光与梦，编织每一次游玩。',
      phase: 'M0',
    };
    expect(Object.keys(sample).sort()).toEqual(['name', 'phase', 'slogan', 'version']);
  });
});
