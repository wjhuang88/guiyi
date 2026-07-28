# 从《归一》抽取成果的实施流程

## 1. 抽取原则

不是复制整个代码库，而是将成果分为四类：

1. **直接迁移**
2. **抽象后迁移**
3. **保留在《归一》**
4. **淘汰并重写**

## 2. 抽取顺序

### Step 1：冻结现有事实

在《归一》仓库创建一个引擎抽取基线标签：

```text
engine-extraction-baseline-YYYYMMDD
```

同时记录：

- 当前 Bevy 版本
- 当前测试数量
- 当前内容 Schema
- 当前 Stage 和 Story 数据
- 当前运行状态模型
- 已知缺陷

### Step 2：建立新仓库骨架

新仓库只建立：

- Workspace
- 文档治理
- CI
- 空 crate
- 版本策略
- License 和贡献规则

不得在此阶段复制大量游戏代码。

### Step 3：抽取稳定基础类型

优先抽取：

- Stable ID
- Hex / Grid 坐标
- Diagnostic
- Content Path
- Version
- Registry 基础接口
- Testkit 基础结构

### Step 4：建立兼容测试

在新仓库中创建 Golden Data：

- 旧 `scenes.ron`
- 旧 `story.ron`
- 旧技能和敌人数据样例

目的不是长期支持旧格式，而是验证转换器。

### Step 5：实现转换器

```text
LegacySceneConfig -> StageDocument
LegacyStoryNode -> DialogueDocument
LegacyEnemyData -> Actor/Unit Definition
```

转换器必须：

- 可重复执行
- 不覆盖人工修改
- 输出诊断
- 记录旧 ID 到新 ID 映射
- 产生迁移报告

### Step 6：抽取 Runtime 能力

按照领域顺序：

1. Stage 生命周期
2. Runtime Content Loader
3. Actor Instance
4. Trigger Runtime
5. Encounter Runtime
6. Dialogue Runtime
7. Persistent Stage State

不要先抽取 UI 和表现层。

### Step 7：建立《归一》适配层

在《归一》仓库新增：

```text
crates/guiyi_engine_adapter/
```

它负责：

- 将 Engine Runtime 事件转换为《归一》事件
- 注册《归一》扩展
- 注册寿元、灵台等项目组件
- 将新 Stage Artifact 实例化为游戏实体

### Step 8：双轨运行

短期内保留：

```text
legacy_content_path
new_engine_content_path
```

通过 Feature 或启动参数切换：

```bash
cargo run --features engine-content
```

双轨期只用于验证，不允许长期并行演进。

### Step 9：迁移首个 Vertical Slice

选择范围最小、依赖清晰的切片：

- 一个 Stage
- 一个 NPC
- 一个 Trigger
- 一个 Encounter
- 一段 Dialogue
- 一个 Stage Connection

### Step 10：切断旧路径

只有满足以下条件后才能删除旧路径：

- 新路径完成端到端测试。
- 存档迁移完成。
- 内容转换报告无阻塞错误。
- 一周内无回退。
- 《归一》Backlog 明确关闭旧实现。

## 3. 单向同步规则

- 引擎基础设施只从引擎仓库发布。
- 《归一》不得反向复制修改后的 Engine Core。
- 发现通用需求时，在引擎仓建立 Story。
- 《归一》临时补丁必须注明：
  - 临时原因
  - 删除条件
  - 对应引擎 Story ID

## 4. 依赖方式

早期：

```toml
guiyi_engine = { git = "...", rev = "<pinned commit>" }
```

稳定后：

- 使用版本 Tag。
- 使用明确 SemVer。
- 禁止跟踪 `main`。
- 每次升级必须有 Upgrade Story 和验证报告。
