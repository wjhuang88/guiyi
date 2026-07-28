# GUIYI Engine 项目章程

## 1. 项目使命

建设一套基于 Bevy 的、面向战术 RPG 与剧情驱动游戏的独立游戏开发基础设施，使游戏团队可以通过稳定的数据契约、编辑能力、预览能力和验证工具持续生产内容，而不必在每个新项目中重复搭建底层工具。

## 2. 核心目标

### 2.1 运行时基础设施

提供：

- 项目和内容包加载
- Stage 生命周期
- 稳定 ID 与跨文档引用
- Definition / Session / Persistent 三层状态模型
- 内容编译和运行实例化
- Preview Runtime
- 存档与内容版本边界

### 2.2 编辑器基础设施

提供：

- Project / Document / Object 模型
- Schema Registry
- Inspector
- Hierarchy
- Viewport
- Selection
- Command / Transaction / Undo / Redo
- Diagnostics
- Extension Registry
- Preview Launcher

### 2.3 类型工具包

第一期支持：

- Stage
- Grid
- Actor Placement
- Trigger
- Stage Connection
- Encounter
- Dialogue
- Quest
- Condition / Effect
- Asset Reference

### 2.4 工具链

提供：

- CLI validate
- CLI compile
- CLI migrate
- CLI package
- CI 门禁
- 内容依赖图
- 构建报告
- 预览启动器

## 3. 非目标

首个稳定版本前不做：

- 通用 3D DCC 工具
- 通用动画编辑器
- 通用 Shader Graph
- 通用行为树编辑器
- Rust 动态库热重载
- 多人实时协同编辑
- 插件市场
- 通用脚本语言
- 自研渲染后端
- 完整替代 Unity 或 Unreal

## 4. 分层定位

```text
Bevy
  ↓
GUIYI Engine Core
  ↓
Tactical RPG Toolkit
  ↓
Game Project Extensions
  ↓
Concrete Game Content
```

## 5. 成功标准

### 5.1 首个 Alpha 成功标准

- 能打开一个项目。
- 能创建和保存 Stage 文档。
- 能放置 Actor、Trigger 和 Stage Connection。
- 能验证错误引用。
- 能点击 Preview，在独立进程中启动 Stage。
- 能通过 CLI 编译内容。
- 《归一》可通过适配层加载一个迁移后的 Stage。

### 5.2 首个 Beta 成功标准

- Stage、Encounter 和 Dialogue 都可编辑和预览。
- 所有编辑操作支持 Undo / Redo。
- 内容可通过严格校验进入构建。
- 《归一》能够使用引擎包完成一个完整 vertical slice。
- 第二个非武侠示例项目无需修改 Engine Core 即可运行。

### 5.3 1.0 成功标准

- API 和内容 Schema 有明确兼容策略。
- 提供稳定版本发布、升级和迁移文档。
- 提供至少两个项目验证。
- 引擎仓库和游戏仓库可独立发布。
- 所有核心模块均有自动化测试和 CI。
- 一线开发可以仅依据文档和 backlog 实施工作。

## 6. 治理原则

- Backlog 是唯一可执行需求入口。
- ADR 是架构决策唯一权威入口。
- Reference 文档只描述当前可信事实。
- Proposal 不得直接指导实施。
- 每个 Story 必须具有验收标准和验证门禁。
- 未通过门禁的任务不得标记 Done。
- 引擎基础层不得引用具体游戏命名空间。
