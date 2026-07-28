# 《归一》成果迁移矩阵

| 现有成果 | 去向 | 策略 | 说明 |
|---|---|---|---|
| Bevy Plugin 组织经验 | Engine Reference | 抽象后迁移 | 形成 PluginGroup 和生命周期规范 |
| `HexCoord` | Toolkit Core | 抽取 | 移除战斗专用耦合 |
| `SceneConfig` | Legacy Converter | 转换 | 不直接作为新 StageDocument |
| `ActiveScene` | Runtime Stage 参考 | 重写 | 当前混合模板和运行状态 |
| `WorldState` | 《归一》项目 | 保留 | 具体游戏世界状态 |
| `GameState` | 《归一》项目 | 保留 | 具体游戏流程 |
| `StoryDatabase` | Legacy Converter | 转换 | 迁移为 Dialogue / Quest 文档 |
| 两套 StoryCondition | Toolkit + GUIYI Extension | 合并重构 | 通用条件在 Toolkit，具体条件在游戏扩展 |
| DataLoaderPlugin | Engine Content Pipeline 参考 | 重写 | 改为验证、编译和 Artifact 加载 |
| SaveLoadPlugin | Engine Persistence Reference | 部分抽取 | 文件位置、版本和接口需重构 |
| Combat System | Tactical RPG Runtime | 选择性抽取 | 先抽接口和 Encounter，不直接复制整个实现 |
| Character Creation | 《归一》项目 | 保留 | 不属于引擎基础能力 |
| Lifespan | 《归一》扩展 | 保留 | 项目特定 |
| Spirit Altar | 《归一》扩展 | 保留 | 项目特定 |
| Asset Slot 文档设计 | Engine Asset | 抽象后迁移 | 形成 AssetId / Slot |
| Backlog / ADR / SOP | Engine Docs | 迁移 | 修改编号和项目定位 |
| 单元测试模式 | Engine Testkit | 抽取 | 增加 App 生命周期测试 |
| 210 个现有测试 | 《归一》仓库 | 保留 | 仅选择通用部分迁移 |
| README 当前闭环描述 | Archive / Migration Baseline | 保留快照 | 不作为新引擎事实 |
