# 质量与验证门禁

## 1. 全局门禁

所有 PR 必须通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run -p engine_cli -- validate --strict
```

## 2. 文档门禁

- Markdown 链接有效。
- Reference 不包含待办清单。
- Backlog 状态与代码一致。
- ADR 状态明确。
- 所有公共 API 有文档。
- 所有 breaking change 有升级说明。

## 3. 架构门禁

每个基础设施任务必须回答：

- 是否让 Core 依赖具体游戏？
- 是否使用 Entity 作为持久引用？
- 是否混淆作者态和运行态？
- 是否直接修改文档而绕过 Command？
- 是否引入第二套权威状态？
- 是否允许非法内容进入 Runtime？
- 是否影响可重复加载和卸载？

任一回答为“是”，默认不得合并。

## 4. 任务类型门禁

### Core 类型

必须具备：

- 单元测试
- API 文档
- 一个最小示例
- 错误路径测试
- 无具体游戏依赖

### Runtime 类型

必须具备：

- 生命周期测试
- 重复进入退出测试
- 资源和实体泄漏检查
- 错误内容拒绝测试
- Headless 集成测试

### Editor 类型

必须具备：

- Undo / Redo 测试
- Dirty State 测试
- Save / Reload 测试
- 崩溃恢复或自动保存测试
- 不通过直接字段写入绕过 Command

### Content Schema 类型

必须具备：

- 正常序列化往返
- 旧版本迁移
- 非法输入诊断
- Golden File
- Reference 文档

### CLI 类型

必须具备：

- Exit Code 测试
- 结构化输出测试
- 无交互模式
- 错误信息稳定性
- CI 使用样例

### 游戏接入类型

必须具备：

- 引擎仓测试通过
- 游戏仓集成测试通过
- 至少一个端到端路径
- 旧路径对比
- 回滚开关

## 5. Done 定义

Story 只有满足以下全部条件才可 Done：

- 验收标准全部勾选。
- 自动化测试通过。
- 手工验证完成。
- 文档同步。
- 无未登记已知缺陷。
- 回滚方式明确。
- 对应迭代验证报告已更新。
