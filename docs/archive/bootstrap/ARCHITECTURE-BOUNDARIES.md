# 架构边界

## 1. 依赖方向

```text
engine_core
  ↑
engine_editor_core
  ↑
engine_editor_ui
  ↑
tactical_rpg_toolkit
  ↑
game_extension
```

禁止：

```text
engine_core -> tactical_rpg_toolkit
engine_core -> guiyi_game
engine_editor_core -> guiyi_game
```

## 2. 建议 Workspace

```text
crates/
├── engine_core
├── engine_content
├── engine_runtime
├── engine_authoring
├── engine_validation
├── engine_asset
├── engine_preview
├── engine_cli
├── engine_editor
├── engine_testkit
├── tactical_rpg_content
├── tactical_rpg_runtime
├── tactical_rpg_editor
└── tactical_rpg_validation
```

## 3. 核心抽象

### Project

负责：

- 项目根目录
- 内容根目录
- 资源根目录
- 启用扩展
- 构建配置

### Document

所有可编辑内容的统一抽象。

### DocumentObject

文档内可被选择、修改和引用的稳定对象。

### Schema

描述对象和字段如何被编辑、校验和序列化。

### Command

所有作者态写操作的唯一入口。

### Diagnostic

统一表达错误、警告和建议。

### Artifact

文档编译后的运行时构建产物。

### Extension

注册文档类型、属性编辑器、工具、校验器、编译器和预览器。

## 4. 状态模型

```text
Definition
  静态内容模板

Session
  本次运行实例

Persistent
  跨运行保存结果
```

禁止直接序列化整个 Bevy World 作为游戏内容或存档权威格式。

## 5. Stage 边界

```text
StageDocument
  作者态，可编辑，可 Git Diff

StageArtifact
  已验证、已解析引用的编译产物

RuntimeStage
  实例化到 ECS 世界后的运行对象
```

## 6. 具体游戏扩展

具体游戏只能通过以下方式扩展：

- 注册新的 Authoring Component
- 注册 Property Editor
- 注册 Validator
- 注册 Compiler
- 注册 Preview Overlay
- 注册 Runtime Plugin
- 注册 Condition / Effect 类型

具体游戏逻辑不得进入 Engine Core。
