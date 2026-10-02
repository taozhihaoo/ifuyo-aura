// app_info 命令的 TS 镜像 —— 与 src-tauri/src/ipc.rs 的 AppInfo DTO 一一对应
// （Rust 侧 serde rename_all = "camelCase"）。修改任一侧必须同步另一侧。
import { invoke } from '@tauri-apps/api/core';

export interface AppInfo {
  name: string;
  version: string;
  slogan: string;
  phase: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('app_info');
}
