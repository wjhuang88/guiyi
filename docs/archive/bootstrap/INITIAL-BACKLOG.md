# 初始 Backlog

## P0：项目建立

### ENG-001 建立新仓库和 Workspace

**目标**：新仓库具备独立构建和发布基础。

**验收标准**

- [ ] Workspace 创建完成。
- [ ] 所有计划 crate 有占位包。
- [ ] CI 执行 fmt、clippy、test、doc。
- [ ] README 说明项目定位。
- [ ] 引擎仓不引用《归一》代码。

**验证**

```bash
cargo check --workspace
cargo test --workspace
```

---

### ENG-002 引入文档治理结构

**验收标准**

- [ ] reference/backlog/iterations/decisions/roadmap/sop/proposals/archive 完成。
- [ ] Backlog 是唯一执行入口。
- [ ] ADR 模板可用。
- [ ] 链接检查进入 CI。

---

### ENG-003 建立版本和发布策略

**验收标准**

- [ ] 定义 SemVer。
- [ ] 定义 Engine API、Content Schema、Save Schema 三种版本。
- [ ] 定义 breaking change 流程。
- [ ] 定义 Upgrade Guide 责任。

---

## P0：基础抽象

### ENG-010 Stable ID 与类型标识

**验收标准**

- [ ] ProjectId / DocumentId / ObjectId / TypeId 可序列化。
- [ ] 不使用 Entity 作为持久 ID。
- [ ] 重复 ID 可诊断。
- [ ] 具备往返测试。

---

### ENG-011 Diagnostic 模型

**验收标准**

- [ ] 支持 error/warning/info。
- [ ] 支持定位文档、对象和字段。
- [ ] 支持 CLI 文本和 JSON 输出。
- [ ] 具备稳定错误码。

---

### ENG-012 Extension Registry

**验收标准**

- [ ] 可注册 Document Type。
- [ ] 可注册 Validator。
- [ ] 可注册 Compiler。
- [ ] 可注册 Property Editor。
- [ ] 注册冲突可诊断。

---

### ENG-013 Document 与 Schema Registry

**验收标准**

- [ ] 可创建、加载、保存自定义文档。
- [ ] 支持版本字段。
- [ ] 支持字段元数据。
- [ ] 支持引用字段。
- [ ] 示例扩展通过测试。

---

### ENG-014 Command / Transaction / Undo / Redo

**验收标准**

- [ ] 创建、删除、属性修改可撤销。
- [ ] Transaction 合并拖拽过程。
- [ ] Redo 栈行为正确。
- [ ] 文档 Dirty State 正确。

---

## P0：Stage 链路

### ENG-020 StageDocument

**验收标准**

- [ ] StageDocument 与运行时 Entity 解耦。
- [ ] 支持对象、图层和连接。
- [ ] 支持自定义 Authoring Component。
- [ ] 具备 Golden File。

---

### ENG-021 Stage Compiler

**验收标准**

- [ ] StageDocument 可编译为 StageArtifact。
- [ ] 所有引用在编译时解析。
- [ ] 非法内容编译失败。
- [ ] Artifact 可稳定重建。

---

### ENG-022 Runtime Stage Lifecycle

**验收标准**

- [ ] 加载和卸载状态明确。
- [ ] StageOwned 实体可完整清理。
- [ ] 重复 100 次无泄漏。
- [ ] Global Persistent 实体不被删除。

---

### ENG-023 Preview Runner

**验收标准**

- [ ] 可指定 Stage 启动。
- [ ] 独立进程运行。
- [ ] 返回 Exit Code。
- [ ] 可输出 Runtime Diagnostics。
- [ ] 编辑器崩溃与 Preview 崩溃隔离。

---

## P1：Editor Core

### ENG-030 Project Browser
### ENG-031 Document Tabs
### ENG-032 Hierarchy
### ENG-033 Inspector
### ENG-034 Viewport Selection
### ENG-035 Autosave 与恢复
### ENG-036 Diagnostics Panel
### ENG-037 Preview Launcher

每项均必须包含：

- [ ] 自动化测试
- [ ] Undo / Redo 验证
- [ ] Save / Reload 验证
- [ ] 示例扩展验证

---

## P1：Tactical RPG Toolkit

### ENG-040 Grid Stage
### ENG-041 Actor Placement
### ENG-042 Trigger
### ENG-043 Stage Connection
### ENG-044 Condition / Effect Registry
### ENG-045 Encounter Document
### ENG-046 Dialogue Document
### ENG-047 Navigation Validator
### ENG-048 Tactical Demo Project

---

## P1：《归一》迁移

### ENG-060 Legacy Scene Converter
### ENG-061 Legacy Story Converter
### ENG-062 GUIYI Engine Adapter
### ENG-063 GUIYI Custom Components
### ENG-064 First Vertical Slice Migration
### ENG-065 Legacy Path Removal
