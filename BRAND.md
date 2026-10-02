# BRAND

> 品牌唯一事实来源是仓库根目录的 [`brand.json`](brand.json)。本文描述其用法与边界，不得与之冲突。

## 品牌事实

| 项 | 值 |
|---|---|
| 名称 | `ifuyo Aura` |
| 厂商 | `ifuyo · 伊芙游` |
| 口号 | 以光与梦，编织每一次游玩。 |
| 官网 | https://ifuyo.com |
| 版权 | Copyright © 2026 ifuyo · 伊芙游 |

气质：干净、克制、富有情绪。

## 命名规则

- 产品名写作 **`ifuyo Aura`**（"ifuyo" 小写 i，"Aura" 大写 A），不得写作 `Ifuyo Aura`、`IFUYO Aura`、`Aura` 单独使用指代产品。
- `aura-*`（aura-core 等）是**内部工程 crate 命名**，允许出现在源码、架构文档、开发者文档中，不得作为用户可见的产品名或 UI 文案。
- 已退役名称：`IFUYO Music`、`Aura Music`、`Aura Player`、`IFY Music`、`IFUYO Player`——不得在任何新内容中出现（Spec 中旧名称审计清单内的引用除外）。

## 颜色

品牌色定义于 `brand.json` 的 `colors`，唯一消费路径：

```text
brand.json → scripts/generate-tokens.mjs → ui/src/generated/tokens.css（CSS variables）
                              ↘ 校验 ui/src/design/tokens.ts 一致
```

| Token | 值 | 用途 |
|---|---|---|
| `bg` | `#0B0E14` | 深色底 |
| `rose` | `#F2789F` | 强调色候选 |
| `violet` | `#8B7CF6` | 强调色候选 |
| `sky` | `#5CC8F2` | 强调色候选 |
| `mint` | `#6FE3C1` | 强调色候选 |

设计规则（Charter §27）：dark、克制、高级；每个时刻只允许一个强调色；禁止巨型阴影、过度玻璃拟态、游戏化奖励 UI。

## 品牌资产边界

- 允许：按名称引用本项目、直接链接 ifuyo.com 或项目仓库、在评测/文章中使用官方截图。
- 不允许：将修改版构建、二次分发版冒充为官方 ifuyo Aura；使用品牌名暗示官方背书；将品牌元素用于无关产品。
- 官方 logo / 图标资产随 M0–M6 建立；在此之前仓库不含任何品牌图形资产。
