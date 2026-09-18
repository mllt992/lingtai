# DESIGN.md — 凌台 · Loft

## 1. Objective

Loft 是一块贴在桌面角落的控制台，不是一间要走进去的房间。打开它应该像瞥一眼手腕上的表：面积小、信息够、不抢焦点。任何界面改动的质量门槛是——关窗之后，用户几乎记不得它刚才占了多少屏幕。

## 2. Product Context

- **What the product does:** 驻留 Windows 桌面的轻量控制台：快速启动、文件归纳、端口监控、性能监控。
- **Who it's for:** 本机开发者 / 重度桌面用户，主显示器上同时开着 IDE、浏览器、终端，Loft 只是随手点一下的工具。
- **Adjacent brands (feel like these):** Rainmeter 小组件、PowerToys Run、Raycast 迷你窗、Windows 11 系统托盘弹出层。
- **Distant brand (do not feel like this):** Notion / Linear 那种全屏工作台——Loft 不是工作本身，是工作旁边的开关。
- **Cultural register:** 技术、克制、工具感。不卖情绪，不装饰。

## 3. Visual Foundations

沿用现有主题系统（`src/styles/themes.css`），本次不换肤。

### 3a. Color

- **Neutral scale（Aurora 默认）:** `--bg #0b1020` / `--bg-soft #0f1730` / `--bg-card #141d3a` / `--bg-elev #1c2649` / `--border #233059` / `--text #e6edff` / `--text-muted #8a96bd`
- **Accent:** `--accent #5b8cff`（用户可覆盖），`--accent-soft rgba(91,140,255,0.16)`
- **Semantic:** `--success #4ade80` / `--warning #fbbf24` / `--danger #f87171`
- **Usage rules:** 强调色只出现在当前 Tab 指示条、主按钮、占用告警；禁止把整块面板刷成 accent 渐变。

### 3b. Typography

- **Display / Body:** `'Inter', 'PingFang SC', 'Microsoft YaHei', system-ui, sans-serif`
- **Data / 等宽数字:** `'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace` + `font-feature-settings: 'tnum'`
- **Type scale（mini 模式收紧）:** 10 / 11 / 12 / 13 / 15 / 16。页面标题 15px/600，分组名 12px/600，图标标签 10.5px，数字 13–14px。
- **Weight discipline:** 正文 400，标签 500，标题 600。禁止 700+ 大标题。

### 3c. Spacing & rhythm

- **Base unit:** 4px
- **Spacing scale:** 4 / 8 / 12 / 16 / 24
- **Mini 密度:** 标题栏 32px；顶栏 Tab 36px；页面内边距 10–12px；卡片圆角仍走 `--radius-md 10px` / `--radius-lg 14px`。不追求「慷慨留白」，mini 模式的空白就是浪费。

### 3d. Component seeds

- **Button:** ghost（描边）+ primary（实心 accent）。高度 mini 32px / expanded 36px。
- **Card:** 现有 `--bg-card` + 1px `--border`，hover 微抬 1–2px。不新增阴影语言。
- **Iconography:** Carbon icons，顶栏 Tab 只用图标 + tooltip；expanded 侧栏保留图标+文字。
- **Window chrome:** 自绘标题栏。mini 不提供最大化；展开/折叠、图钉、最小化、关闭并排在右上。

## 4. Accessibility

- **Text contrast:** 正文 ≥ 4.5:1；muted 文案只用于次要说明。
- **Motion:** 模式切换只做窗口几何动画 180ms `ease-out`；内容不飞入。尊重 `prefers-reduced-motion`：直接切尺寸、不 tween。
- **Focus indicators:** 2px accent 描边，不 `outline: none` 后无替代。
- **Hit target:** 顶栏图标按钮 ≥ 28×28；启动器图标卡片在 mini 仍可点（约 72×72 含文字）。
- **Alt text:** logo 装饰性可空；功能按钮必须有 `title`。

## 5. Voice & Tone

- **Register:** 技术、短句、命令式。
- **Sentence rhythm:** 短。按钮两个字：「展开」「折叠」「置顶」。
- **Words this brand uses:** 展开 / 折叠 / 置顶 / 启动 / 收纳。
- **Words this brand refuses:** 沉浸式、无缝、赋能、工作台、仪表盘、魔法。
- **Address:** 直接对用户下指令，不用「您」。

## 6. Implementation Practices

- **Token format:** 现有 CSS 变量；新增 `html[data-ui-mode="mini"]` 作为密度开关，不另起一套主题。
- **Component library:** 继续 Vue SFC + UnoCSS + Carbon icons，不引入新 UI 库。
- **Grid:** mini 启动器 `minmax(72px, 1fr)`；资源列表改单列；监控卡片单列；端口表横向滚动。
- **Motion:** 窗口 `setSize` + `setPosition` 同步完成，前端 CSS 不模拟窗体缩放。
- **Window geometry:** 逻辑像素。mini 400×560（最小 360×440）；expanded 980×680（最小 760×520）。首次贴工作区右下，边距 12px。

## 7. Anti-Patterns

- 不做全屏工作台：默认打开不能再是 1180×760。
- 不做悬浮启动坞（一排图标）：用户明确要双模式面板，不是 dock。
- 不做透明亚克力 / 渐变 hero / 大数字三卡片墙（U1/U5）。
- 不把监控页塞进 HUD：HUD 继续只报 CPU/MEM/TEMP/VPN。
- 不在 mini 里保留 76px 左栏——那是 expanded 的骨架。
- 不把「最大化」留在 mini 标题栏：那会一键毁掉 mini 的意义。

## 8. Decision-Making

遇到不确定的视觉选择时，问：关窗之后，用户的 IDE 还在不在正中间？如果答案是「被挡住了」，选更小的那个。

## 9. Workflow

先改窗体几何与壳层（TitleBar / Sidebar / App.vue），再给各 View 加 mini 密度覆盖。业务 store 与 Rust 命令不动。弹层（添加应用 720×560）在 mini 下改为贴窗体的全幅 sheet，避免撑破 400px 窗。
