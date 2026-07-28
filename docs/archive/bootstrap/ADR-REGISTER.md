# ADR 建议清单

## 必须在 Phase 0 批准

### ADR-0001 项目分层和依赖方向

决定 Engine Core、Toolkit 和 Game Extension 的边界。

### ADR-0002 作者态、编译态和运行态分离

决定：

```text
Document -> Artifact -> Runtime Instance
```

### ADR-0003 稳定 ID 策略

决定禁止使用 Entity 或文件路径作为长期身份。

### ADR-0004 静态编译扩展模型

首期使用 Rust 静态注册，不使用动态 DLL 插件。

### ADR-0005 独立 Preview 进程

编辑器不直接嵌入正式 Runtime World。

## Phase 1

### ADR-0006 Schema Registry 与 Bevy Reflect 的关系
### ADR-0007 Diagnostic 错误码策略
### ADR-0008 文档版本和迁移策略
### ADR-0009 Artifact 缓存和确定性构建
### ADR-0010 Command 和 Transaction 模型

## Phase 2–4

### ADR-0011 Stage 生命周期
### ADR-0012 Authoring Component 编译协议
### ADR-0013 Condition / Effect 扩展协议
### ADR-0014 资产 ID 与 Slot 策略
### ADR-0015 Editor UI 框架和 Dock 方案

## Phase 5+

### ADR-0016 游戏项目依赖引擎的版本策略
### ADR-0017 存档兼容边界
### ADR-0018 内容包和 DLC 边界
