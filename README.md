# 凌台 · Loft

> 轻量级 Windows 桌面控制台 —— 快速启动 / 资源归纳 / 端口监控 / 性能监控，一个窗口搞定。

```
凌台 = 一方凌然而立的台面。
Loft = 干净、开阔的高层空间。
```

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3-42b883)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-stable-orange)](https://www.rust-lang.org)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)

## 特性

- 🚀 **启动器** —— 自动扫描开始菜单 + 手动添加 + 拖拽即收纳，搜索一键启动
- 📂 **资源归纳** —— 文件夹 / 文件 / 网址 三合一卡片，双击直达
- 📊 **性能监控** —— CPU / 内存 / 交换 / 各核心 / 各磁盘 / GPU（NVIDIA 优先，WMI 兜底）实时折线
- 🔌 **端口监控** —— 列出用户进程 LISTEN 端口，过滤系统噪声，一键定位 / 终止占用
- 🎨 **6 套主题** —— Aurora 极光 · Carbon 碳灰 · Nord 北欧 · Vanilla 米白 · Solar 暖橙 · Mint 薄荷
- 🪶 **轻量原生** —— Tauri 2 + Rust，安装包 ~10 MB，运行内存 ~40 MB

## 截图（占位）

> 主题切换后整页毛玻璃质感会同步切换。

```
┌─────────────────────────────────────────────────┐
│ 凌台 · Loft                       ─  ▢  ✕   │  ← 自绘标题栏
├──┬──────────────────────────────────────────────┤
│启│ 启动器                  搜索 [_____]  +添加 │
│资│ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐         │
│监│ │ VSC  │ │Chrome│ │Figma │ │ ...  │         │
│端│ └──────┘ └──────┘ └──────┘ └──────┘         │
│设│                                              │
└──┴──────────────────────────────────────────────┘
```

## 技术架构

| 层 | 选型 | 说明 |
|---|---|---|
| 应用框架 | **Tauri 2** | WebView 前端 + Rust 后端，原生性能、极小体积 |
| 前端 | **Vue 3 + TypeScript + Pinia + Vue Router** | Composition API，类型完整 |
| 样式 | **UnoCSS** + CSS Variables | 原子化 + 主题切换走 `[data-theme]` |
| 系统信息 | **sysinfo** | 纯 Rust 跨平台，CPU / RAM / Disk / Process |
| 端口枚举 | **netstat2** | 跨平台 TCP/UDP 套接字枚举 |
| GPU | **nvml-wrapper**（NVIDIA） / **WMI**（兜底） | 双路径，按可用性自动降级 |
| 快捷方式解析 | **lnk** | 解析 `.lnk` 取真实目标 / 图标 |
| 资源打开 | **tauri-plugin-opener** | 统一走系统 shell |

## 快速开始

### 依赖

- Node.js ≥ 20
- pnpm ≥ 9
- Rust ≥ 1.77
- Windows: 默认 MSVC 工具链（Visual Studio Build Tools 2022）
- macOS / Linux: 参考 [Tauri 先决条件](https://tauri.app/start/prerequisites/)

### 开发

```powershell
pnpm install         # 同时生成默认占位图标
pnpm tauri:dev       # 启动开发模式（自带 HMR）
```

首次启动 Rust 端编译约 5–10 分钟，之后增量编译秒级。

### 构建

```powershell
pnpm tauri:build     # 生成 .msi / .exe 安装包到 src-tauri/target/release/bundle/
```

NVIDIA GPU 监控可选启用：

```powershell
pnpm tauri build --features gpu-nvml
```

### 一些命令

```powershell
pnpm icons           # 重新生成占位图标
pnpm build           # 仅前端构建（vue-tsc + vite）
pnpm dev             # 仅前端开发服务器（不带 Tauri 壳）
```

## 目录结构

```
.
├── CLAUDE.md                 # 项目协作纲领（AI 自动加载）
├── README.md                 # 本文件
├── package.json
├── vite.config.ts
├── uno.config.ts
├── tsconfig.json
├── index.html
├── docs/                     # 工作流 / 规范 / 设计文档（参见 docs/规范套件说明.md）
│   └── 02_功能设计/
│       └── 凌台Loft_实施方案_20260513.md
├── scripts/
│   └── gen-icons.mjs         # 零依赖图标生成（PNG + ICO + ICNS）
├── src/                      # Vue 前端
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   ├── stores/               # Pinia: settings / launcher / monitor / ports
│   ├── views/                # 5 个主视图
│   ├── components/           # TitleBar / Sidebar / StatCard / SparkLine ...
│   ├── composables/          # useDrop 等
│   ├── styles/               # themes.css + main.css
│   └── types/
└── src-tauri/                # Rust 后端
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── state.rs
        └── commands/
            ├── launcher.rs   # 扫描开始菜单 / 解析 lnk / 启动
            ├── files.rs      # 打开路径 / 网址 / 资源管理器定位
            ├── monitor.rs    # CPU / 内存 / 磁盘 / GPU 快照
            ├── ports.rs      # 端口枚举 / 进程关联 / kill
            └── settings.rs   # 配置 / 启动项 / 资源 JSON 读写
```

## 主题预设

| ID | 名称 | 调性 | 模式 | 强调色 |
|---|---|---|---|---|
| `aurora` | 极光 | 深蓝渐变 + 青紫高光 | 深色 · 默认 | `#5b8cff` |
| `carbon` | 碳灰 | 纯黑哑光 + 橙红强调 | 深色 · OLED | `#ff6b35` |
| `nord` | 北欧 | 冷灰蓝 + 雪青 | 深色 | `#88c0d0` |
| `vanilla` | 米白 | 暖白 + 棕灰 | 浅色 | `#b07b3a` |
| `solar` | 暖橙 | 米色 + 暖橙 | 浅色 | `#cb4b16` |
| `mint` | 薄荷 | 浅青绿 + 薄荷 | 浅色 | `#16a085` |

用户可在 *设置 → 自定义主色* 覆盖任意主题的强调色（`--accent`），所有按钮 / 高亮 / 折线会同步。

## 数据存放

| 路径 | 内容 |
|---|---|
| `%APPDATA%\com.loft.app\config.json` | 主题 / 主色 / 刷新间隔 / 启动器选项 |
| `%APPDATA%\com.loft.app\items.json` | 用户添加的启动项 + 资源（文件 / 文件夹 / 网址） |

## 隐私

凌台完全离线运行，不连接任何外部服务，所有数据保存在本机。

## 路线图

- [x] M1 主框架 + 启动器 + 资源 + 监控 + 端口 + 主题（v0.1）
- [ ] M2 系统托盘 + 全局快捷键 + 开机自启
- [ ] M3 macOS / Linux 适配
- [ ] M4 插件机制（用户自写卡片类型）

## 许可

MIT
