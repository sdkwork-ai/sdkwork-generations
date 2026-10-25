> Migrated from `docs/directory-structure-verification.md` on 2026-06-24.
> Owner: SDKWork maintainers

## 项目根目录结构

```
sdkwork-generations/
├── .sdkwork/                    # ✅ 工作空间元数据
├── apis/                        # ✅ API合约
├── apps/                        # ✅ 应用目录
├── configs/                     # ✅ 配置模板
├── crates/                      # ✅ Rust crates
├── deployments/                 # ✅ 部署描述
├── docs/                        # ✅ 文档
├── examples/                    # ✅ 示例
├── jobs/                        # ✅ 作业定义
├── plugins/                     # ✅ 插件
├── scripts/                     # ✅ 脚本
├── sdks/                        # ✅ SDK工作空间
├── specs/                       # ✅ 规范
├── storage/                     # ✅ 存储
├── tests/                       # ✅ 测试
└── tools/                       # ✅ 工具
```

## 应用目录结构

### 1. PC应用 (`apps/sdkwork-generations-pc/`)

**标准**: `APP_PC_ARCHITECTURE_SPEC.md`

```
sdkwork-generations-pc/
├── .sdkwork/                    # ✅ 工作空间元数据
│   ├── README.md                # ✅
│   ├── skills/README.md         # ✅
│   └── plugins/README.md        # ✅
├── AGENTS.md                    # ✅ 代理入口
├── CLAUDE.md                    # ✅ Claude兼容性垫片
├── GEMINI.md                    # ✅ Gemini兼容性垫片
├── CODEX.md                     # ✅ Codex兼容性垫片
├── agent.app.config.json      # ✅ 应用清单
├── package.json                 # ✅ 包配置
├── specs/                       # ✅ 本地规范
├── src/                         # ✅ 源代码
│   ├── App.tsx
│   ├── main.tsx
│   └── bootstrap/
└── tests/                       # ✅ 测试
```

**验证结果**: ✅ 符合标准

---

### 2. H5应用 (`apps/sdkwork-generations-h5/`)

**标准**: `APP_H5_ARCHITECTURE_SPEC.md`

```
sdkwork-generations-h5/
├── .sdkwork/                    # ✅ 工作空间元数据
│   ├── README.md                # ✅
│   ├── skills/README.md         # ✅
│   └── plugins/README.md        # ✅
├── AGENTS.md                    # ✅ 代理入口
├── CLAUDE.md                    # ✅ Claude兼容性垫片
├── GEMINI.md                    # ✅ Gemini兼容性垫片
├── CODEX.md                     # ✅ Codex兼容性垫片
├── agent.app.config.json      # ✅ 应用清单
├── package.json                 # ✅ 包配置
├── README.md                    # ✅ 文档
├── index.html                   # ✅ 入口HTML
├── tsconfig.json                # ✅ TypeScript配置
├── tsconfig.node.json           # ✅ Node TypeScript配置
├── vite.config.ts               # ✅ Vite配置
├── bin/                         # ✅ 运维脚本
│   ├── ios/
│   └── android/
├── config/                      # ✅ 配置模板
│   ├── browser/
│   ├── host/
│   ├── server/
│   └── container/
├── docs/                        # ✅ 文档
├── public/                      # ✅ 静态资源
├── scripts/                     # ✅ 脚本
├── sdks/                        # ✅ SDK工作空间
├── specs/                       # ✅ 本地规范
├── src/                         # ✅ 源代码
│   ├── App.tsx
│   ├── AuthGate.tsx
│   ├── main.tsx
│   ├── index.css
│   └── bootstrap/
│       ├── environment.ts
│       ├── hostAdapters.ts
│       ├── iamRuntime.ts
│       ├── routes.ts
│       ├── runtime.ts
│       └── sdkClients.ts
└── tests/                       # ✅ 测试
```

**验证结果**: ✅ 符合标准

---

### 3. Flutter应用 (`apps/sdkwork-generations-flutter-mobile/`)

**标准**: `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md`

```
sdkwork-generations-flutter-mobile/
├── .sdkwork/                    # ✅ 工作空间元数据
│   ├── README.md                # ✅
│   ├── skills/README.md         # ✅
│   └── plugins/README.md        # ✅
├── AGENTS.md                    # ✅ 代理入口
├── CLAUDE.md                    # ✅ Claude兼容性垫片
├── GEMINI.md                    # ✅ Gemini兼容性垫片
├── CODEX.md                     # ✅ Codex兼容性垫片
├── agent.app.config.json      # ✅ 应用清单
├── pubspec.yaml                 # ✅ Dart包配置
├── README.md                    # ✅ 文档
├── config/                      # ✅ 配置模板
│   ├── app/
│   ├── host/
│   ├── server/
│   └── container/
├── docs/                        # ✅ 文档
├── lib/                         # ✅ Dart源代码
│   ├── app.dart
│   ├── auth_gate.dart
│   ├── main.dart
│   └── bootstrap/
│       ├── environment.dart
│       ├── host_adapters.dart
│       ├── iam_runtime.dart
│       ├── routes.dart
│       ├── runtime.dart
│       └── sdk_clients.dart
├── scripts/                     # ✅ 脚本
├── sdks/                        # ✅ SDK工作空间
├── specs/                       # ✅ 本地规范
└── test/                        # ✅ 测试
```

**验证结果**: ✅ 符合标准

---

## 规范对齐验证

| 检查项 | PC应用 | H5应用 | Flutter应用 |
|--------|--------|--------|-------------|
| `.sdkwork/` 目录 | ✅ | ✅ | ✅ |
| `AGENTS.md` | ✅ | ✅ | ✅ |
| `CLAUDE.md` | ✅ | ✅ | ✅ |
| `GEMINI.md` | ✅ | ✅ | ✅ |
| `CODEX.md` | ✅ | ✅ | ✅ |
| `agent.app.config.json` | ✅ | ✅ | ✅ |
| 包配置文件 | ✅ | ✅ | ✅ |
| 本地规范目录 | ✅ | ✅ | ✅ |
| 源代码目录 | ✅ | ✅ | ✅ |
| 测试目录 | ✅ | ✅ | ✅ |
| 配置模板目录 | N/A | ✅ | ✅ |
| 文档目录 | N/A | ✅ | ✅ |
| 脚本目录 | N/A | ✅ | ✅ |
| SDK工作空间 | N/A | ✅ | ✅ |
| Bootstrap目录 | ✅ | ✅ | ✅ |

## 补充文件验证

| 文件 | PC应用 | H5应用 | Flutter应用 |
|------|--------|--------|-------------|
| `.gitignore` | ✅ | ✅ | ✅ |
| `pnpm-workspace.yaml` | ✅ | ✅ | N/A |
| `analysis_options.yaml` | N/A | N/A | ✅ |
| `vite.config.ts` | ✅ | ✅ | N/A |
| `tsconfig.json` | ✅ | ✅ | N/A |
| `index.html` | ✅ | ✅ | N/A |

## 应用清单验证

| 应用 | app.key | app.name | app.appType |
|------|---------|----------|-------------|
| PC | `sdkwork-generations-pc` | SDKWork Generations PC | `APP_REACT` |
| H5 | `sdkwork-generations-h5` | SDKWork Generations H5 | `APP_REACT` |
| Flutter | `sdkwork-generations-flutter-mobile` | SDKWork Generations Flutter Mobile | `APP_FLUTTER` |

## 结论

所有三个应用都已按照sdkwork-specs标准规范完成目录结构对齐：

1. **PC应用**: 符合 `APP_PC_ARCHITECTURE_SPEC.md` 标准
2. **H5应用**: 符合 `APP_H5_ARCHITECTURE_SPEC.md` 标准
3. **Flutter应用**: 符合 `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` 标准

每个应用都包含：
- 标准工作空间元数据 (`.sdkwork/`)
- 代理入口文件 (`AGENTS.md`)
- 工具兼容性垫片 (`CLAUDE.md`, `GEMINI.md`, `CODEX.md`)
- 应用清单 (`agent.app.config.json`)
- 包配置文件 (`package.json` 或 `pubspec.yaml`)
- 标准目录结构 (`src/` 或 `lib/`, `config/`, `specs/`, `tests/` 等)
- 忽略文件 (`.gitignore`)
- 工作空间配置 (`pnpm-workspace.yaml` 或 `analysis_options.yaml`)

**验证状态**: ✅ 完全对齐
**验证时间**: 2026-06-14
