# 开发 SOP

## 1. 文档分层

```text
docs/
├── reference/    当前可信事实
├── backlog/      唯一可执行需求池
├── iterations/   当前与历史迭代
├── decisions/    ADR
├── roadmap/      阶段方向
├── sop/          标准流程
├── proposals/    未批准方案
└── archive/      历史材料
```

## 2. 需求进入流程

```text
Idea
→ Proposal
→ Architecture / Product Review
→ Backlog Story
→ Ready
→ Iteration
→ In Progress
→ Validation
→ Done
```

任何开发任务没有 Story ID 不得开始。

## 3. Story Ready 条件

一个 Story 进入 Ready 前必须具备：

- 用户价值或技术目标
- 明确范围
- 明确不做
- 验收标准
- 依赖
- 风险
- 影响模块
- 验证命令
- 回滚方式
- 文档更新范围

## 4. 分支策略

推荐：

```text
main
feature/ENG-xxx-short-name
fix/ENG-xxx-short-name
docs/ENG-xxx-short-name
```

规则：

- 一个分支只对应一个主 Story。
- 不直接提交 main。
- PR 必须关联 Story。
- 不允许“顺手重构”扩大范围。
- 破坏性变更必须关联 ADR。

## 5. 提交规范

```text
feat(stage): ENG-021 add stage artifact compiler
fix(editor): ENG-044 preserve selection after undo
docs(adr): ENG-010 approve document model
test(runtime): ENG-029 cover repeated stage reload
```

## 6. PR 必备内容

- 变更摘要
- Story ID
- 范围和不做
- 验收结果
- 执行过的命令
- 测试证据
- 迁移影响
- 风险
- 截图或日志
- 文档更新
- 回滚方法

## 7. 代码评审顺序

1. 边界是否正确
2. 状态所有权是否清楚
3. 是否引入项目特定依赖
4. 生命周期是否可重复
5. 错误是否可诊断
6. 测试是否覆盖验收标准
7. 文档是否同步
8. 风格和局部实现

## 8. 发布 SOP

```text
Freeze
→ 全量验证
→ 生成迁移报告
→ 版本号裁决
→ Release Candidate
→ 样例项目验证
→ 《归一》升级验证
→ Tag
→ 发布包
→ Release Notes
→ 升级指南
```
