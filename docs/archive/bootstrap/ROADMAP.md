# GUIYI Engine 完整路线图

## 总览

| 阶段 | 目标 | 预计周期 |
|---|---|---:|
| Phase 0 | 项目建立与治理落地 | 1 周 |
| Phase 1 | 核心抽象与最小工具链 | 2–4 周 |
| Phase 2 | Stage Runtime 与 Preview | 3–5 周 |
| Phase 3 | Editor Core MVP | 4–6 周 |
| Phase 4 | Tactical RPG Toolkit MVP | 4–7 周 |
| Phase 5 | 《归一》接入与迁移 | 3–6 周 |
| Phase 6 | 内容生产 Beta | 5–8 周 |
| Phase 7 | 稳定化与 1.0 | 6–10 周 |

周期为单名熟悉 Rust / Bevy 的核心工程师估算；多人并行时仍需保留架构和集成串行工作。

---

## Phase 0：项目建立与治理

### 目标

- 新仓库可以独立构建。
- 文档治理和 SOP 生效。
- 《归一》仓库与引擎仓库的责任边界确定。

### 任务

- 创建 Workspace。
- 建立文档目录。
- 建立 Backlog。
- 建立 ADR。
- 建立 CI。
- 建立版本策略。
- 建立贡献和分支策略。
- 建立与《归一》的集成分支和依赖方式。

### 出口门禁

- `cargo check --workspace` 通过。
- `cargo test --workspace` 通过。
- `cargo fmt --check` 通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- 文档链接检查通过。
- ADR-0001 至 ADR-0005 已批准。

---

## Phase 1：核心抽象与最小工具链

### 目标

建立不依赖任何游戏类型的基础模型。

### 交付

- ProjectDescriptor
- DocumentId / ObjectId
- Stable Type ID
- Diagnostic
- Extension Registry
- Document Registry
- Schema Registry
- CLI 框架
- 文档版本和迁移接口

### 出口门禁

- 可创建一个自定义文档类型。
- 可加载、保存和迁移文档。
- 可注册并运行 Validator。
- 可通过 CLI 输出结构化诊断。
- 引擎基础 crate 不依赖 tactical RPG 或《归一》。

---

## Phase 2：Stage Runtime 与 Preview

### 目标

建立 StageDocument → StageArtifact → RuntimeStage 的完整链路。

### 交付

- StageDocument 基础格式
- Stage Compiler
- Stage Artifact
- Runtime Stage Loader
- StageOwned 生命周期
- Preview Runner
- Runtime Diagnostics
- 独立 Preview 进程

### 出口门禁

- 同一 Stage 可重复加载和卸载 100 次，无实体泄漏。
- Preview 可从命令行启动指定 Stage。
- 非法引用不能进入 Runtime。
- Stage Artifact 可由 clean checkout 重建。
- Runtime 不读取作者态编辑器私有字段。

---

## Phase 3：Editor Core MVP

### 目标

建立通用编辑器框架。

### 交付

- Project Browser
- Document Tabs
- Hierarchy
- Inspector
- Selection
- Command Stack
- Undo / Redo
- Dirty State
- Autosave
- Diagnostics Panel
- Preview Launcher
- Extension Registration

### 出口门禁

- 所有编辑动作通过 Command。
- Undo / Redo 覆盖创建、删除、移动和属性编辑。
- 编辑器崩溃后可恢复最近自动保存。
- Inspector 不依赖具体游戏类型。
- 一个示例扩展可以新增文档和属性编辑器。

---

## Phase 4：Tactical RPG Toolkit MVP

### 目标

支持一类战术 RPG 的基础内容生产。

### 交付

- Grid Stage
- Actor Placement
- Trigger
- Stage Connection
- Encounter Definition
- Dialogue Document
- Condition / Effect Registry
- Navigation Validation
- Tactical Preview Profiles

### 出口门禁

- 可制作一个非武侠战术 Demo。
- 不修改 Engine Core 即可添加新的 Condition / Effect。
- Stage、Encounter、Dialogue 引用均可跨文档验证。
- Preview 能从 Actor、Encounter、Dialogue 三种入口启动。

---

## Phase 5：《归一》接入与迁移

### 目标

让《归一》成为第一个真实消费项目。

### 交付

- `guiyi_engine_adapter`
- 旧 SceneConfig 转换器
- 旧 StoryDatabase 转换器
- 运行时 Stage 接入
- 《归一》自定义 Condition / Effect
- 《归一》自定义 Authoring Component
- Vertical Slice 迁移

### 出口门禁

- 《归一》主仓不再直接加载旧 `scenes.ron`。
- 至少一个 Stage 使用新引擎内容产物。
- 探索 → 战斗 → 剧情 → 场景切换可完成。
- 存档可保存 Stage Persistent State。
- 游戏项目不需要引用 Editor crate。

---

## Phase 6：内容生产 Beta

### 目标

让关卡、剧情和美术能够使用工具链生产内容。

### 交付

- Asset Registry
- Asset Slot
- Thumbnail
- Dependency Graph
- Localization
- Build Profiles
- Content Reports
- Story Graph
- Encounter Editor
- Batch Validation

### 出口门禁

- 非程序人员可完成一个内容切片。
- 所有缺失引用可在编辑器中定位。
- 资源替换不要求修改 Stage 文档。
- CI 可以验证全部内容。
- 构建报告可追踪每个 Stage 的依赖。

---

## Phase 7：稳定化与 1.0

### 目标

发布可独立演进的首个稳定版本。

### 交付

- API 稳定策略
- Schema 兼容策略
- 存档兼容策略
- Release Notes
- Upgrade Guide
- 性能基准
- 崩溃报告
- 两个样例项目
- 完整 Reference 和 SOP

### 出口门禁

- 连续三个版本无破坏性迁移事故。
- 第二个游戏项目接入无需修改 Core。
- 核心路径具备集成和回归测试。
- 所有公开扩展点有示例和文档。
- 发布包可以由 CI 自动生成。
