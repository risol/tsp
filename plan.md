# TSP 从 Deno Fork 迁移到 Bun Fork 的详细改造计划

> 目标：将 `risol/tsp` 从当前的 Deno 定制运行时迁移到一个**不依赖 `bun --hot`、不修改 JavaScriptCore、支持按 TSP 页面依赖子图定向失效与重新实例化**的 Bun Fork。
>
> 本计划面向长期维护的 TSP Runtime，而不是一次性的 Bun 兼容移植。
>
> 基线日期：2026-08-24

---

## 1. 执行摘要

当前 TSP 为了实现 `.tsp` 直接执行和嵌套依赖热更新，对 Deno/denort 做了较深的 Runtime Patch，核心包括：

- 将 `.tsp` 映射成 TSX；
- 修改 Deno module loader；
- 修改 standalone/VFS 动态模块加载；
- 增加 `--dynamic-import-no-cache`；
- 对动态 import 注入 `?__tsp_v=...` generation token；
- 将 token 向子依赖传播，以绕过 V8 的 ESM module registry；
- 运行时从真实文件系统读取外部 `.tsp/.ts/.tsx`，而不是只从 compiled VFS 读取。

这套方案能够工作，但它的本质是**通过改变 module identity 创建新的模块代际**，而不是对同一个 canonical module key 做真正的失效和重新实例化。

迁移到 Bun 后，不应简单改成：

```bash
bun --hot src/main.ts
```

也不应继续：

```ts
import(path + "?v=" + Date.now());
```

目标应该是给 Bun Fork 增加一个 TSP 专用 Runtime：

```text
Persistent Bun Runtime
├── HTTP Server
├── Session Store
├── DB / Redis / LDAP
├── Logger
├── DI Registry
└── TSP Module Runtime
    ├── native .tsp => TSX
    ├── dependency graph
    ├── reverse dependency graph
    ├── targeted module invalidation
    ├── page generation
    ├── atomic reload
    ├── last-known-good generation
    └── real-filesystem dynamic module loading
```

最终 TSP 主代码应该从当前的：

```ts
const pageModule = await import(fileUrl);
```

演进为类似：

```ts
const pageModule = await Bun.TSP.loadPage(filepath);
```

模块缓存、依赖追踪、失效、转译和 generation 管理全部下沉到 Bun Fork。

---

# 2. 改造目标

## 2.1 必须实现的目标

### G1. 不修改 JavaScriptCore

允许修改：

- Bun Rust Runtime；
- Bun bundler / transpiler；
- Bun module loader；
- Bun watcher；
- Bun 自己的 C++/JSC glue（仅在确有必要时）。

禁止修改：

- WebKit / JavaScriptCore 源码；
- JSC GC / parser / bytecode / ModuleRecord 实现。

JSC 应被视为固定预编译依赖。

### G2. 不依赖 `bun --hot`

TSP Server 正常启动：

```bash
./tspserver
```

Runtime 本身长期常驻。

文件变化只影响相关 TSP page graph，不触发：

- 进程 restart；
- VM 全局 soft reload；
- 全量 `Loader.registry` reset；
- 全量源码 re-evaluate。

### G3. 相同 URL / 相同 canonical module key 可以重新加载

这是最核心的技术目标。

禁止通过：

```text
/foo.ts?v=1
/foo.ts?v=2
```

实现热更新。

必须允许：

```text
file:///www/foo.ts
```

在失效后重新生成新的 module instance。

### G4. 嵌套依赖任意深度热更新

例如：

```text
page.tsp
  -> Wrapper.tsx
     -> Component.tsx
        -> Utils.ts
```

修改 `Utils.ts` 后，下一次访问 `page.tsp` 必须使用新代码。

### G5. Persistent Runtime 和 Reloadable Page Graph 严格分离

以下状态不得因页面代码变化重建：

- HTTP listener；
- SessionStore；
- Redis client/pool；
- MySQL pool/client factory；
- LDAP infrastructure；
- logger；
- config state；
- server-level DI registry。

### G6. 支持单文件 `tspserver` + 外部 `www/`

发布结构继续保持：

```text
tspserver.exe
config.jsonc
www/
  index.tsp
  components/
  ...
```

`www/` 必须是**真实文件系统上的 mutable source tree**，而不是 executable 内嵌 bundle。

### G7. Windows / Linux 至少作为一等平台

特别验证：

- Windows 路径 canonicalization；
- 原子保存/rename；
- watcher 行为；
- 文件被编辑器短暂删除再创建；
- case-insensitive path；
- executable + external source tree。

---

# 3. 明确的非目标

第一阶段不要解决所有 JavaScript HMR 问题。

不做：

- 通用 Vite `import.meta.hot`；
- 任意 npm package 的 HMR；
- Node.js 通用 module hot swapping；
- 任意模块的 dispose 生命周期；
- 修改 JSC ModuleRecord 内部结构；
- 浏览器端 React Fast Refresh；
- 自动保存用户模块全局状态。

TSP 的模型应该刻意受限：

> 页面模块是 transient；长期资源必须由 TSP Runtime/DI 持有。

这能显著降低 HMR 的复杂度。

---

# 4. 当前 TSP 架构需要保留和删除的部分

## 4.1 保留

当前 TSP 中以下设计具有价值，应继续保留：

- `.tsp` 作为 HTTP route page；
- `.tsp` 可以 import `.ts/.tsx/.tsp`；
- `Page(async (ctx, deps) => ...)`；
- dependency injection；
- Request/Response 风格 API；
- React SSR；
- session abstraction；
- schema-first DB API；
- config auto reload；
- static file handling；
- security/path boundary；
- 当前 hot reload E2E 测试思想。

## 4.2 迁移完成后删除

最终应删除：

```text
deno/                  # Deno submodule
deno_tsp_patch.diff
deno-tsp/
```

删除所有只为 Deno fork 服务的：

- `deno-tsp` build；
- `denort-tsp` build；
- `DENORT_BIN`；
- Deno standalone VFS patch；
- `dynamic_import_no_cache`；
- `__tsp_v`；
- Deno-specific sysroot build pipeline。

---

# 5. 目标架构

## 5.1 Runtime 分层

```text
┌────────────────────────────────────────────┐
│                 tspserver                  │
├────────────────────────────────────────────┤
│ Persistent Runtime                         │
│                                            │
│  Bun.serve                                 │
│  Router                                    │
│  Config                                    │
│  SessionStore                              │
│  Logger                                    │
│  DB / Redis / LDAP                         │
│  DI Registry                               │
│                                            │
├──────────────── TSP Boundary ──────────────┤
│ TSP Module Runtime                         │
│                                            │
│  PageRegistry                              │
│  ModuleGraph                               │
│  ReverseDependencyGraph                    │
│  DirtySet                                  │
│  GenerationManager                         │
│  RealFilesystemLoader                      │
│  TargetedRegistryInvalidator               │
│                                            │
├────────────────────────────────────────────┤
│ Bun Runtime / Transpiler / Resolver         │
├────────────────────────────────────────────┤
│ JavaScriptCore (unmodified)                │
└────────────────────────────────────────────┘
```

## 5.2 Page Graph 示例

```text
/a.tsp
  ├── components/Header.tsx
  │     └── lib/date.ts
  └── lib/auth.ts

/b.tsp
  └── components/Header.tsx
        └── lib/date.ts
```

正向关系：

```text
a.tsp -> Header.tsx
Header.tsx -> date.ts
b.tsp -> Header.tsx
```

反向关系：

```text
date.ts -> Header.tsx
Header.tsx -> a.tsp
Header.tsx -> b.tsp
```

当 `date.ts` 改变：

```text
date.ts
   ↑
Header.tsx
   ↑       ↑
a.tsp    b.tsp
```

结果：

```text
mark a.tsp dirty
mark b.tsp dirty
```

不要立即执行所有页面。

下一次请求 `/a` 时只 rebuild/re-instantiate A 所需 generation。

---

# 6. Bun Fork 管理策略

## 6.1 Repository 建议

推荐保留两个仓库：

```text
risol/bun-tsp-runtime     # fork oven-sh/bun
risol/tsp                 # TSP framework/server
```

或者继续直接 fork `oven-sh/bun`，但建立明确分支：

```text
upstream/main
master or tsp-main
feature/tsp-module-loader
feature/tsp-module-graph
feature/tsp-targeted-invalidation
feature/tsp-standalone
```

## 6.2 Patch 原则

所有 TSP 变更尽量满足：

1. 新增代码优于侵入式修改；
2. TSP-specific 代码集中；
3. upstream rebase 冲突面尽量小；
4. 不复制 Bun 已有 resolver/transpiler；
5. 复用已有 watcher/module cache machinery；
6. 不 fork WebKit/JSC。

推荐新增目录：

```text
src/runtime/tsp/
  mod.rs
  module_graph.rs
  page_registry.rs
  loader.rs
  invalidation.rs
  generation.rs
  api.rs
```

如果 Bun crate layering 不允许直接放在 `src/runtime/tsp`，则按 Bun 当前 crate 边界拆分，但仍保持 `tsp_*` 命名集中。

---

# 7. Bun 源码优先勘探点

基于当前 Bun 源码，首轮应重点阅读和打断点/日志的区域：

```text
src/jsc/hot_reloader.rs
src/jsc/VirtualMachine.rs
src/jsc/RuntimeTranspilerStore.rs
src/watcher/Watcher.rs
src/runtime/jsc_hooks.rs
src/runtime/cli/run_command.rs
src/bundler/options.rs
src/bundler/ParseTask.rs
```

其中：

- `src/jsc/hot_reloader.rs`
  - 已负责 watcher 到 reload task 的调度；
  - 可参考 changed-path 传递方式；
  - TSP 不直接使用全局 reload，但可复用 watcher plumbing。

- `src/jsc/VirtualMachine.rs`
  - 当前已有 VM hot reload 入口；
  - 要追踪全量 reload 最终清理哪些 registry/cache；
  - 找到可以拆成 targeted operation 的最小边界。

- `src/jsc/RuntimeTranspilerStore.rs`
  - 重点确认 runtime transpilation cache key；
  - targeted invalidation 时需要同步处理旧 transpilation artifacts。

- `src/watcher/Watcher.rs`
  - 复用 Bun 原生文件 watcher；
  - TSP watcher 只产生 dirty signal，不直接执行 reload。

- `src/bundler/options.rs` / parser 相关代码
  - 原生 `.tsp -> TSX` extension mapping。

**注意：以上是优先勘探点，不应在验证调用链之前假定最终所有文件都需要修改。**

---

# 8. Phase 0 — 建立可重复的 Bun Fork 开发基线

## 目标

确保可以在 Windows 和 Linux 上稳定：

- build Bun fork；
- 修改 Rust；
- 使用官方 prebuilt JSC；
- 跑 targeted tests；
- 不构建 WebKit/JSC。

## 工作项

### P0-01 固定 upstream commit

记录：

```text
BUN_UPSTREAM_COMMIT=<sha>
WEBKIT_VERSION=<bun upstream value>
```

禁止自行固定与 upstream 不匹配的 JSC。

### P0-02 建立 fork patch inventory

新增：

```text
docs/tsp-runtime-patches.md
```

记录每一个 TSP patch：

```text
Patch ID
涉及文件
目的
是否可能 upstream
是否依赖 Bun 内部 API
rebase 风险
```

### P0-03 CI 最小矩阵

至少：

```text
Windows x64 debug
Linux x64 debug
```

后续再补 release。

## Exit Criteria

- 能修改 Bun Rust 后生成新 `bun`/`bun.exe`；
- 能跑一个新加的 Rust/JS runtime test；
- 构建过程完全没有 WebKit source build。

---

# 9. Phase 1 — Targeted Module Invalidation 技术 PoC

这是整个迁移的 **Go / No-Go Gate**。

在这一步成功之前，不开始大规模 port TSP。

## 9.1 PoC 目标

验证：

> 同一个 canonical file URL 在 registry eviction 后，可以被重新 resolve/transpile/instantiate/evaluate，并返回新的 exports。

### 测试 A：单模块同 URL 重载

初始：

```ts
// foo.ts
export const version = "v1";
```

执行：

```ts
const a = await import(fileUrl);
assert(a.version === "v1");
```

磁盘改为：

```ts
export const version = "v2";
```

调用内部测试 API：

```ts
Bun.__internalInvalidateModule(fileUrl);
```

再次：

```ts
const b = await import(fileUrl);
assert(b.version === "v2");
assert(a !== b);
```

必须满足：

```text
specifier A == specifier B
```

不能使用 query/hash cache bust。

## 9.2 PoC 需要识别的所有缓存

不能只清一个 JS `Map`。

需要审计：

```text
ES module registry
CommonJS require cache（若 TSP graph 允许 CJS）
runtime transpiler cache
resolved source cache
source map cache
resolver caches
watcher file metadata
package.json resolution cache
plugin loader cache（若有）
```

第一版 TSP graph 可以明确禁止或限制 CJS，以减少状态空间。

## 9.3 建议内部原语

先不要发布公共 API。

Rust 内部可以形成类似：

```rust
pub struct ModuleInvalidationResult {
    pub registry_removed: bool,
    pub transpiler_cache_removed: bool,
    pub resolver_cache_removed: bool,
}

fn invalidate_runtime_module(
    vm: &mut VirtualMachine,
    specifier: &CanonicalModuleSpecifier,
) -> ModuleInvalidationResult;
```

JavaScript test-only binding：

```ts
Bun.__internalInvalidateModule(path);
```

生产版本最终可以完全不暴露这个 low-level API。

## 9.4 Test B：父子模块

```text
page.ts -> dep.ts
```

修改 `dep.ts` 后只失效 `dep.ts`，再次 import `page.ts`。

预期：**仍然得到旧结果。**

这个测试非常重要，因为它证明：

> 单点 eviction 不足以解决 ESM graph reload。

这应作为后续 reverse-dependent invalidation 的基线测试。

## Exit Criteria

- 相同 URL 单模块 reload 成功；
- 明确列出所有必须同步清理的 Bun Runtime cache；
- 没有修改 JSC；
- 没有 query token；
- 没有 VM 全量 reset。

如果这一步无法稳定实现，再重新评估 Bun Fork 路线。

---

# 10. Phase 2 — `.tsp` Native Loader

## 10.1 目标

让 Bun Runtime 原生认识：

```text
.tsp = TSX source
```

而不是依赖 JS plugin：

```ts
onLoad({ filter: /\.tsp$/ }, ...)
```

## 10.2 要求

以下路径都必须一致识别 `.tsp`：

- resolver；
- loader selection；
- parser；
- runtime transpiler；
- dependency scanner；
- sourcemap；
- watcher；
- stack trace source filename；
- standalone external loader。

## 10.3 Loader 语义

`.tsp` 应采用 TSX parser：

```text
syntax: TypeScript + JSX
```

JSX 配置必须与 TSP 目前 React SSR 行为兼容。

## 10.4 测试

```text
import root .tsp
.tsp imports .ts
.tsp imports .tsx
.tsp imports another .tsp
.ts imports .tsp       # 明确是否允许
.tsx imports .tsp      # 明确是否允许
syntax error location
source map stack trace
Windows path
```

建议继续兼容当前 TSP 规则：

```text
.tsp 可以 import .tsp/.ts/.tsx
.ts/.tsx 是否允许 import .tsp 需要做明确产品决定
```

如果希望减少 page root 识别复杂度，可以规定：

> `.tsp` 是 page/root-capable module；普通 `.ts/.tsx` 是 library module。

## Exit Criteria

以下代码无需 plugin：

```ts
await import("file:///.../index.tsp");
```

并可以正常执行 JSX/TS。

---

# 11. Phase 3 — TSP Module Graph

不要复用 Bun `--hot` 的“整个 entry graph”作为 TSP graph。

建立专用 graph。

## 11.1 数据结构

建议概念模型：

```rust
type ModuleId = u32;
type PageId = u32;

struct TspModuleNode {
    id: ModuleId,
    canonical_path: PathBuf,

    dependencies: SmallVec<ModuleId>,
    dependents: SmallVec<ModuleId>,

    owning_pages: SmallVec<PageId>,

    source_version: SourceVersion,
    mtime_ns: u64,
    size: u64,

    state: ModuleState,
}

enum ModuleState {
    Clean,
    Dirty,
    Loading,
    Failed,
}

struct TspPageSlot {
    id: PageId,
    root_module: ModuleId,

    current_generation: Option<GenerationId>,
    next_generation: GenerationId,

    dirty: bool,
    loading: bool,

    last_error: Option<ReloadError>,
}
```

具体类型应根据 Bun 内部 allocator/collection 约束调整。

## 11.2 Canonical Path 是关键不变量

必须定义统一 canonicalization：

```text
relative path
   -> absolute
   -> normalize separators
   -> resolve symlink policy
   -> Windows drive normalization
   -> case normalization policy
   -> canonical ModuleKey
```

禁止 graph 和 JSC registry 使用两套不一致 key。

Windows 上尤其验证：

```text
C:\www\foo.ts
c:\www\foo.ts
C:/www/foo.ts
file:///C:/www/foo.ts
```

最终必须映射到同一个 ModuleKey。

## 11.3 Graph 更新

每次成功 transpile/resolve 一个 TSP graph module 时，收集其 static dependencies：

```text
A -> B
A -> C
```

同步维护：

```text
B.dependents += A
C.dependents += A
```

旧版本 dependency 发生变化时必须先移除旧 edge。

## 11.4 Dynamic Import 策略

需要明确：

```ts
await import(variable)
```

如何加入 graph。

建议：

- resolved 到 `www/` TSP source boundary 内的动态文件模块：运行时加入 graph；
- node_modules：默认视为 persistent/external dependency，不做 TSP HMR；
- `https:` remote import：第一阶段禁止或 external；
- `data:` 等特殊 specifier：不进入 TSP file graph。

## Exit Criteria

能够打印/debug dump：

```text
TSP Graph
Page /a.tsp
  /a.tsp
    -> /components/a.tsx
       -> /lib/x.ts
```

并能够从任意 leaf 找到所有 owning pages。

---

# 12. Phase 4 — Reverse Dependency Invalidation

## 12.1 核心原则

发生变化时向**父方向**传播 dirty：

```text
changed leaf
   ↑
parent
   ↑
page root
```

不是简单向下 invalidate dependency tree。

## 12.2 算法

概念伪代码：

```rust
fn mark_changed(graph: &mut Graph, changed: ModuleId) {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back(changed);

    while let Some(id) = queue.pop_front() {
        if !visited.insert(id) {
            continue;
        }

        graph[id].state = Dirty;

        for page in graph[id].owning_pages {
            page_registry[page].dirty = true;
        }

        for parent in graph[id].dependents {
            queue.push_back(parent);
        }
    }
}
```

## 12.3 不要变化时立即重新执行 page

Watcher event 只做：

```text
changed path
   -> canonical module id
   -> mark dirty
```

真正 reload 在下一次请求触发。

这避免：

- 一次保存触发大量无意义编译；
- 用户正在连续编辑时反复 evaluate；
- 无人访问页面消耗 CPU；
- 多页面共享 dependency 变化造成 reload storm。

## 12.4 Shared Dependency

```text
a.tsp -> shared.ts
b.tsp -> shared.ts
```

`shared.ts` 改变：

```text
a dirty
b dirty
```

请求 A 后：

```text
A -> new generation
B -> still dirty
```

请求 B 时再构建 B。

## Exit Criteria

现有 TSP nested dependency hotreload test 可以在**完全没有 URL token**的情况下通过。

---

# 13. Phase 5 — Page Generation 模型

仅有 cache eviction 还不足以做稳定服务器。

必须加入 generation。

## 13.1 目标

允许：

```text
Request #1 -> Generation 10
source changes
Request #2 -> Generation 11
Request #1 continues safely on Generation 10
```

## 13.2 Generation 生命周期

```text
CURRENT
  Generation N
      |
source dirty
      |
build candidate N+1
      |
  +---+---+
  |       |
fail    success
  |       |
keep N   atomic publish N+1
```

## 13.3 不要主动销毁正在执行的旧 generation

旧 generation 只能：

```text
no new requests
      ↓
reference count becomes zero / JS references disappear
      ↓
GC
```

不要对正在执行的 ModuleRecord 做强制 free。

## 13.4 Generation 对象

概念上：

```rust
struct PageGeneration {
    id: GenerationId,
    root_module_key: ModuleKey,
    module_keys: Vec<ModuleKey>,
    namespace: StrongJsReference,
    created_at: Instant,
}
```

是否需要保存 `namespace` strong reference，要根据 Bun/JSC 当前 module namespace 生命周期决定。

## Exit Criteria

并发测试：

```text
request A starts old generation
modify dependency
request B sees new generation
request A completes correctly
```

无 crash、use-after-free、交叉 binding。

---

# 14. Phase 6 — Atomic Reload + Last Known Good

这是 TSP 面向 AI 编码尤其重要的能力。

AI/编辑器写文件过程中，源码可能短暂不完整。

## 14.1 禁止流程

```text
change detected
 -> destroy current page
 -> compile new source
 -> compile error
 -> page unavailable
```

## 14.2 正确流程

```text
current generation N
       |
source changed
       |
build candidate N+1
       |
resolve/transpile/link/evaluate
       |
   success ?
    /    \
  no      yes
  |        |
retain N  atomic publish N+1
```

## 14.3 Dev / Production 策略

### Dev

建议请求直接展示 candidate compile error，同时保存旧 generation 供诊断/选择。

可配置：

```jsonc
{
  "reload": {
    "onCompileError": "show-error"
  }
}
```

### Production

建议：

```jsonc
{
  "reload": {
    "onCompileError": "last-known-good"
  }
}
```

即：

- 记录错误；
- current generation 不变；
- 下一次文件变化继续尝试。

## 14.4 原子性

同一个 PageSlot 同一时刻只允许一个 candidate build。

其他请求：

方案 A：继续使用 current generation；

方案 B：等待 candidate。

推荐默认 A：

> reload 不阻断现有服务。

## Exit Criteria

测试：

1. v1 正常；
2. 写入 syntax error；
3. server 不崩；
4. 修复成 v2；
5. 下一次请求看到 v2。

---

# 15. Phase 7 — Watcher Integration

TSP 不使用 `bun --hot`，但可以复用 Bun watcher。

## 15.1 Watcher 职责必须保持简单

Watcher 只负责：

```text
filesystem event -> changed path
```

Watcher 不负责：

- module eviction；
- dependency graph traversal；
- transpile；
- evaluate；
- page generation publish。

## 15.2 推荐事件管线

```text
OS watcher
   ↓
Bun Watcher
   ↓
ChangedPathQueue
   ↓
TSP graph lookup
   ↓
mark dirty
```

## 15.3 编辑器 Atomic Save

必须处理典型：

```text
write temp
rename old
rename temp -> real path
```

以及：

```text
delete
create
```

不能只依赖 inode/fd identity。

## 15.4 mtime/stat fallback

Watcher 不是正确性的唯一来源。

建议在 dirty-sensitive load path 做低成本校验：

```text
mtime_ns
size
optional fast hash
```

用途：

- watcher 丢事件补偿；
- network filesystem；
- Windows 特殊编辑器行为。

不建议每个请求递归 stat 整棵树。

只检查当前 page graph 已登记 source metadata。

## Exit Criteria

在 Windows/Linux 上连续快速保存 leaf dependency，不发生漏 reload。

---

# 16. Phase 8 — TSP Runtime API

## 16.1 第一版不要公开通用 invalidate API

不要优先设计：

```ts
Bun.invalidateModule(path)
```

这会很快变成通用 HMR API，扩大维护范围。

推荐 TSP-specific 内部 API。

### 候选 API

```ts
Bun.TSP.loadPage(filepath)
```

返回：

```ts
interface TspLoadedPage {
  default: (context: unknown) => unknown | Promise<unknown>;
}
```

或者直接：

```ts
const pageFn = await Bun.TSP.loadPage(filepath);
```

## 16.2 建议内部 API

调试期可以增加：

```ts
Bun.TSP.inspectGraph()
Bun.TSP.inspectPage(filepath)
Bun.TSP.markDirty(filepath)
Bun.TSP.stats()
```

只在 debug build / internal namespace 暴露。

### 示例

```ts
console.log(Bun.TSP.inspectPage("/www/a.tsp"));
```

输出：

```json
{
  "generation": 12,
  "dirty": false,
  "modules": [
    "/www/a.tsp",
    "/www/components/a.tsx",
    "/www/lib/x.ts"
  ]
}
```

这会极大降低后续调试难度。

---

# 17. Phase 9 — Compiled Executable 与 Real Filesystem Boundary

这是第二个重大技术 Gate。

## 17.1 TSP 需要的语义

`tspserver` 自身可以 compile/bundle：

```text
framework/runtime -> executable
```

但：

```text
www/*.tsp/*.ts/*.tsx -> external filesystem
```

必须明确区分：

```text
Embedded Application Runtime Graph
vs
External TSP Page Graph
```

## 17.2 禁止外部页面被 build-time bundle 捕获

例如：

```ts
Bun.TSP.loadPage(filepath)
```

这个 path 必须保持 runtime dynamic external source 语义。

不要让 bundler 把 `www/` 全扫描进 executable。

## 17.3 RealFilesystemLoader

建议给 TSP Runtime 建明确入口：

```rust
load_external_tsp_source(path)
```

而不是让“VFS first / disk fallback”散落在多个 loader 分支。

规则：

```text
TSP source root => always real filesystem
framework/runtime => normal Bun compiled graph
node_modules policy => explicitly defined
```

## 17.4 node_modules 策略

必须尽早决定发行方式。

候选 A：framework dependencies 全内嵌，页面只通过 DI 使用；推荐。

候选 B：允许 page import 本地 node_modules；需要 runtime resolver 和发布 node_modules。

TSP 当前理念是“No ad-hoc imports / DI first”，因此推荐：

> TSP 页面尽量不直接 import npm package；稳定第三方能力通过 DI 提供。

这样 standalone external graph 会简单很多。

## Exit Criteria

生成：

```text
tspserver.exe
www/index.tsp
www/components/a.tsx
```

启动后：

- 可以访问 page；
- 修改 `a.tsx`；
- 不重启 executable；
- 下次请求得到新结果。

---

# 18. Phase 10 — 将 TSP TypeScript Runtime 从 Deno API 迁移到 Bun/Web/Node API

只有 Bun Runtime 核心 PoC 通过后再做这一阶段。

## 18.1 API 替换表

| 当前 | 目标 |
|---|---|
| `Deno.serve` | `Bun.serve` |
| `Deno.stat` | `node:fs/promises.stat` / Bun internal helper |
| `Deno.readFile` | `Bun.file` 或 `node:fs/promises.readFile` |
| `Deno.readTextFile` | `Bun.file(path).text()` |
| `Deno.writeTextFile` | `Bun.write` / node fs |
| `Deno.remove` | node fs |
| `Deno.env.get` | `process.env` |
| `Deno.args` | `Bun.argv` / `process.argv` |
| `Deno.exit` | `process.exit` |
| `Deno.Command` | `Bun.spawn` |
| `Deno.execPath` | `process.execPath` / Bun equivalent |
| `Deno.build.os` | `process.platform` |
| `std/path` | `node:path` |
| JSR jsonc | npm package或内部 parser |

Web 标准部分一般保持：

```text
Request
Response
Headers
URL
URLSearchParams
fetch
Web Crypto
TextEncoder/TextDecoder
```

## 18.2 HTTP 层

当前：

```ts
Deno.serve(...)
```

迁移：

```ts
Bun.serve({
  port,
  async fetch(req) {
    return handleRequest(req, config, logger);
  }
});
```

不要把 Bun HTTP handler 自己纳入 TSP reload graph。

## 18.3 第三方依赖

当前很多依赖本身已经来自 npm：

```text
react
react-dom
zod
nanoid
mysql2
redis
ldapts
exceljs
```

优先直接使用 Bun npm compatibility。

特别回归：

- mysql2；
- redis；
- ldapts；
- exceljs；
- bcryptjs。

## 18.4 Remote HTTP Import

当前存在例如：

```ts
import("https://esm.sh/bcryptjs@...")
```

迁移时建议删除 remote runtime import，改成本地 npm dependency。

理由：

- standalone 更可控；
- 离线运行；
- module graph 更稳定；
- 不把 remote URL 混入 TSP HMR graph。

---

# 19. Phase 11 — Persistent DI / Resource Ownership 重构

这一阶段是为了确保 page generation 可以安全回收。

## 19.1 资源所有权原则

```text
TSP Runtime owns resources
Page owns request-local values only
```

## 19.2 Persistent

放在 Runtime：

```text
SessionStore
Redis connections/pool
DB pool/factory infrastructure
Logger
Config manager
FileManager state
crypto service
TSP runtime graph
```

## 19.3 Request Local

放在 Page execution context：

```text
ctx
cookies
response helper
query/body
request scoped transaction
request scoped logger context
```

## 19.4 Page Module 禁止的模式

文档应明确不推荐/禁止：

```ts
const db = await connect(...);        // module global
setInterval(...);                     // module global timer
emitter.on(...);                      // persistent listener
process.on(...);                      // persistent listener
```

推荐：

```ts
export default Page(async (ctx, { db, logger, session }) => {
  ...
});
```

## 19.5 `globalThis.Page`

当前实现把 `Page` 放到 globalThis，并通过 closure 引用 DI Map。

迁移时建议重构为一个永久 Runtime-owned binding：

```text
globalThis.Page
   -> stable native/JS bridge
   -> Persistent DI Registry
```

不要让 `globalThis.Page` 捕获属于某个 reloadable generation 的 JS module-local Map。

建议：

```ts
const runtimeState = Symbol.for("tsp.runtime.state");
```

或直接由 Bun.TSP native state 持有 registry handle。

## Exit Criteria

Page generation 替换不会：

- 清 session；
- 清 Redis；
- 清 logger；
- 复制 DI builder；
- 产生旧/new `depBuilders` 混用。

---

# 20. Phase 12 — Config Reload

Config reload 与 page code reload 不应共用机制。

当前基于 mtime 的 config reload 可以保留思想。

分成：

```text
ConfigManager
TspModuleManager
```

配置变化：

```text
config file changed
 -> parse candidate
 -> validate candidate
 -> atomic publish config
```

根据字段决定：

- 可热更新；
- 需要重建某个服务；
- 需要 server restart。

不要把 config 文件伪装成 JS module。

---

# 21. Phase 13 — 测试计划

## 21.1 Runtime Invalidation 单元测试

### R1 同 URL 单模块

```text
v1 -> invalidate -> same URL -> v2
```

### R2 Parent stale 基线

只 invalidate child，验证 parent 仍 stale。

用于证明 reverse invalidation 的必要性。

### R3 Reverse graph

```text
A -> B -> C
change C
invalidate C/B/A
import A => new C
```

### R4 Shared dependency

```text
A -> X
B -> X
change X
A/B dirty
```

### R5 Circular dependency

```text
A -> B -> C -> A
```

BFS/DFS 不死循环。

### R6 Dynamic import

```text
page -> dynamic import lib
```

运行后 graph 补 edge。

### R7 Deleted dependency

文件删除后给稳定 module-not-found，不 crash。

### R8 Rename dependency

旧 edge 正确移除。

---

## 21.2 Generation 测试

### G1 Concurrent old/new

旧请求执行期间发布新 generation。

### G2 Compile failure

candidate 失败不破坏 current。

### G3 Runtime evaluation failure

源码 syntactically valid，但 top-level throw。

需要决定：

```text
top-level throw => candidate publish failure
```

推荐不 publish。

### G4 Top-level await

candidate module 有 TLA。

必须等待完成后才能 atomic publish。

### G5 Never-resolving TLA

需要 cancellation/timeout policy，不能永久锁 PageSlot loading。

---

## 21.3 Watcher 测试

- normal save；
- rapid 20 saves；
- atomic rename save；
- delete/create；
- Windows VS Code save；
- network/shared folder（如果产品需要）；
- changed file outside root 不触发；
- node_modules change 默认不触发 TSP graph reload。

---

## 21.4 TSP 现有 E2E 全量迁移

必须迁移现有：

```text
HTTP
injection
validation
session
redis session
MySQL
Redis
LDAP
ExcelJS
upload
security
config
hotreload
```

当前 nested hot reload test 应成为核心 regression test。

---

## 21.5 Memory / GC 测试

连续修改同一个模块，例如 10,000 次：

```text
reload
request
reload
request
...
```

监控：

- RSS；
- JSC heap；
- module count；
- graph node count；
- generation count；
- strong reference count。

目标：旧 generation 可以回收，内存不能线性增长。

不要在每次 page reload 后强制 full GC；可在测试工具中手动 GC 验证可回收性。

---

# 22. Source Boundary 与安全

TSP loader 必须强制 document root。

例如：

```text
root = C:\site\www
```

允许：

```text
C:\site\www\a.tsp
C:\site\www\components\x.tsx
```

禁止：

```text
C:\site\secret.ts
C:\Windows\...
../../secret
```

## 22.1 Symlink Policy

必须明确。

建议默认：

> resolved real path 仍必须位于 configured TSP source roots 内。

否则 symlink 可以绕过 path traversal 检查。

## 22.2 Page 直接 import npm 的权限

建议默认继续强调 DI，而不是 unrestricted import。

可设计：

```jsonc
{
  "modules": {
    "allowNpmImports": false,
    "allowRemoteImports": false
  }
}
```

---

# 23. 性能设计目标

不要一开始追逐极限性能，先保证语义正确。

但设计时保留以下目标。

## 23.1 Clean Page Fast Path

理想流程：

```text
request
 -> route
 -> PageSlot dirty == false
 -> current generation
 -> execute
```

不应：

- 每次重新 parse；
- 每次重新 walk whole graph；
- 每次 hash 全部依赖。

## 23.2 Dirty Page Path

只处理受影响 page graph。

不要：

```text
reset all Loader.registry
re-transpile whole server
```

## 23.3 Graph Shared Node

依赖节点应去重：

```text
shared.ts -> one graph node
```

而不是每 page copy 一份 graph metadata。

Generation 的 module instance 可以独立，但 metadata graph 应共享 canonical node。

---

# 24. 可观测性

没有 observability，HMR bug 会很难排查。

建议增加环境变量：

```text
TSP_DEBUG_MODULES=1
TSP_DEBUG_RELOAD=1
```

输出：

```text
[TSP] changed: /www/lib/x.ts
[TSP] dirty page: /www/a.tsp
[TSP] dirty page: /www/b.tsp
[TSP] reload /www/a.tsp gen=17 -> candidate=18
[TSP] evict module /www/a.tsp
[TSP] evict module /www/components/a.tsx
[TSP] evict module /www/lib/x.ts
[TSP] publish /www/a.tsp gen=18
```

增加 stats：

```text
pages
modules
clean pages
dirty pages
active generations
reload success
reload failure
compile duration
last error
```

---

# 25. 错误模型

需要定义稳定的错误类型：

```text
TspResolveError
TspParseError
TspTranspileError
TspLinkError
TspEvaluateError
TspSecurityError
TspReloadConflict
```

错误中保存：

```text
page root
failed module
specifier
importer
source location
generation
```

Dev 页面显示链路：

```text
/index.tsp
 -> /components/A.tsx
 -> /lib/x.ts:18:4
```

---

# 26. 并发控制

即使 Bun 单 JS thread，watcher/loader/background tasks 仍可能存在跨线程事件。

## 26.1 原则

所有 graph mutation 最终串行化到 VM owner thread。

Watcher thread 不直接修改 JS/module registry。

Watcher 只投递：

```rust
ChangedPathEvent
```

VM thread：

```text
consume event
 -> graph mutation
 -> mark dirty
```

## 26.2 同一 Page reload 去重

```text
request A sees dirty
request B sees dirty almost same time
```

只允许一个 candidate build。

另一个请求默认继续用 current generation。

---

# 27. TSP API 改造后的请求流程

目标请求流程：

```text
HTTP Request
   ↓
route pathname
   ↓
security check
   ↓
resolve .tsp filepath
   ↓
Bun.TSP.loadPage(filepath)
   │
   ├── clean
   │     ↓
   │   current generation
   │
   └── dirty
         ↓
      build candidate
         ↓
      success?
      /     \
    no       yes
    |         |
 error/LKG   publish
      \       /
       ↓     ↓
       pageFn
         ↓
build request context
         ↓
pageFn(ctx)
         ↓
render / Response
```

---

# 28. Repository 级实施顺序

建议严格按以下顺序。

## M0 — Bun Fork Build Baseline

交付：

- Bun fork 可编译；
- Windows/Linux debug CI；
- JSC prebuilt 固定。

## M1 — Same-URL Eviction PoC

交付：

- internal single-module invalidate；
- 同 URL v1→v2 test；
- cache inventory。

**M1 是第一道 Go/No-Go Gate。**

## M2 — Native `.tsp` Loader

交付：

- `.tsp -> TSX`；
- sourcemap/error tests。

## M3 — TSP Module Graph

交付：

- dependency/reverse dependency；
- page ownership；
- graph inspection。

## M4 — Reverse Invalidation

交付：

- nested dependency reload；
- shared dependency dirty propagation。

## M5 — Generation + Atomic Publish

交付：

- current/candidate generation；
- LKG；
- concurrent request test。

## M6 — Watcher

交付：

- Bun native watcher dirty signal；
- Windows/Linux atomic save tests；
- stat fallback。

## M7 — Standalone External Source

交付：

```text
tspserver(.exe) + www/
```

真正热更新。

**M7 是第二道 Go/No-Go Gate。**

## M8 — TSP Framework Port

交付：

- Deno APIs 全部替换；
- Bun HTTP server；
- npm dependencies 验证。

## M9 — Persistent DI/State Refactor

交付：

- reload 不影响 session/DB/logger；
- `globalThis.Page` 稳定。

## M10 — Full Regression

交付：

- 全 E2E；
- memory stress；
- Windows/Linux package tests。

## M11 — Remove Deno

交付：

- 删除 Deno submodule/patch/build pipeline；
- docs 全部切换 Bun-TSP Runtime。

## 28.1 当前实施状态（2026-08-24）

以下状态反映当前 `bun` 分支已经实际完成并验证的范围：

- [x] M0：Bun Fork 已完成 Windows 编译，继续使用 upstream 预编译 JSC。
- [x] M1：same-URL targeted module eviction 已完成并通过测试。
- [x] M2：`.tsp` native loader 已完成，支持 TSX 页面加载。
- [x] M3：TSP dependency/reverse-dependency graph 已接入页面 reload。
- [x] M4：nested/shared dependency 的定向失效与重新实例化已完成。
- [ ] M5：旧请求完成、新 generation 并发隔离的完整验证仍待补齐。
- [ ] M6：Windows watcher 已验证；Linux atomic-save/stat fallback 测试仍待补齐。
- [x] M7：Windows `tspserver.exe + www/` 外部目录热更新已验证。
- [x] M8：TSP 运行时已迁移到 Bun API；React/JSX、React DOM server 以及运行时 npm 包由 Bun Fork 内置 namespace 提供，外部 `www` 不需要携带 `dist/node_modules`。
- [ ] M9：session、Redis/DB/logger 和 config 的持久化 reload 语义仍待完成。
- [ ] M10：Windows 单元测试与 E2E 已通过；memory stress 和 Linux package tests 仍待补齐。
- [x] M11：Deno/denort 相关源码、构建产物和旧说明已清理。

本轮验证记录：`sh tsp.sh check` 通过；93 个单元测试通过；Windows E2E 通过 32/32；使用不带 `node_modules` 的外部 `www` 启动 `tspserver.exe`，React 页面成功进入业务重定向流程。

---

# 29. 建议的 Git Commit / PR 粒度

不要做一个巨大 PR。

推荐：

```text
01 runtime: add targeted module eviction test harness
02 runtime: invalidate runtime transpiler cache by module key
03 runtime: support same-key ESM re-instantiation
04 loader: add native .tsp as TSX source
05 tsp: add module graph metadata
06 tsp: collect dependency and reverse-dependency edges
07 tsp: add page registry and dirty propagation
08 tsp: add generation slots
09 tsp: add atomic candidate publish
10 tsp: integrate native file watcher
11 tsp: add real-filesystem external loader for compiled executable
12 tsp: expose internal loadPage API
```

TSP repo单独：

```text
01 runtime: add Bun compatibility abstraction
02 runtime: replace std/path with node:path
03 runtime: replace Deno filesystem APIs
04 runtime: migrate server to Bun.serve
05 runtime: switch page import to Bun.TSP.loadPage
06 runtime: migrate process/env APIs
07 deps: remove remote imports
08 tests: port unit tests
09 tests: port E2E
10 build: replace Deno compile pipeline
11 build: package tspserver + www
12 cleanup: remove Deno fork
```

---

# 30. 风险清单

## Risk 1 — Bun 的 module registry 不支持安全的单 key eviction

**严重度：最高。**

Mitigation：Phase 1 先做 PoC，不提前 port TSP。

如果 Bun 当前 JSC bridge 只能全局 reset，需要研究是否能在 Bun 自己的 loader registry 层让相同 key 重新生成新的 module record，而不是修改 JSC。

## Risk 2 — 父模块仍保持旧 child binding

这是 ESM 的正常行为。

Mitigation：reverse-dependent closure 全量 eviction 到 page root。

## Risk 3 — 旧 generation 无法 GC

可能因为：

- global reference；
- event listener；
- timer；
- module cache 残留；
- strong namespace reference 未释放。

Mitigation：stress test + heap/module counters + page code ownership规则。

## Risk 4 — Bun compiled executable 对 external runtime TS/TSX import 语义不匹配

Mitigation：Phase 9/M7 单独做技术 Gate；必要时为 TSP 实现显式 RealFilesystemLoader。

## Risk 5 — Windows watcher/path identity

Mitigation：canonical path policy + watcher + stat fallback + Windows 原生 CI。

## Risk 6 — upstream Bun 更新频繁导致 fork 难维护

Mitigation：

- patch 小而集中；
- TSP-specific 新文件优先；
- upstream pin；
- 定期 rebase；
- patch inventory；
- 不碰 JSC。

## Risk 7 — npm package compatibility

Mitigation：业务 port 后逐个 E2E；第三方能力尽量由 persistent DI 层封装。

## Risk 8 — top-level side effects

Reload page module会再次执行 top-level code。

Mitigation：TSP coding model 明确：page/library module top-level 应尽量 pure；长期 side effects 只能在 Runtime DI 层。

---

# 31. 成功标准

整个迁移完成时必须同时满足：

### Runtime

- [x] 不使用 `bun --hot`
- [x] 不使用 `bun --watch` 做 page reload
- [x] 不修改 JavaScriptCore
- [x] 不使用 query-string module cache bust
- [x] 相同 canonical URL 可以得到新 module generation
- [x] nested dependency 任意深度可 reload
- [x] shared dependency 可以标记多个 page dirty
- [x] 只 reload affected page graph
- [ ] 旧请求可以完成
- [ ] 新请求使用新 generation
- [x] reload compile failure 不摧毁 last-known-good

### TSP

- [x] `.tsp` native TSX
- [x] `Bun.TSP.loadPage()` 替换 dynamic import hack
- [ ] session reload 后保持
- [ ] Redis/DB/logger 保持
- [ ] config reload 保持
- [x] static file 行为保持
- [x] security tests 保持
- [x] current nested hotreload E2E 通过

### Packaging

- [x] Windows `tspserver.exe + www/`
- [ ] Linux `tspserver + www/`
- [x] 修改外部 `.tsp/.ts/.tsx` 无需重启 server
- [x] executable 不需要用户安装完整 Bun runtime
- [x] 不需要 Deno/denort

### Maintenance

- [ ] Bun patch inventory 完整
- [ ] fork 可以持续 rebase upstream
- [x] JSC 保持 upstream Bun 预编译版本

---

# 32. 最优的第一批实际任务

如果现在开始实施，不要先迁 `src/main.ts`。

第一批任务应该严格限定为：

```text
1. Fork Bun / 固定 upstream commit
2. 编译 debug Bun，确认 prebuilt JSC workflow
3. 阅读 VirtualMachine::reload 的完整调用链
4. 找出 hot reload 全量清 ES module registry 的真正底层操作
5. 把它拆出一个 test-only targeted module invalidation 原语
6. 写 same URL v1 -> v2 test
7. 写 parent -> child stale baseline test
8. 实现 reverse dependent closure 的最小 prototype
9. 写 A -> B -> C 修改 C 后 A 更新的 test
10. 再开始 .tsp native loader
```

其中第 6 项通过以后，方案从“理论可行”进入“Bun runtime 技术可行”。

第 9 项通过以后，TSP 当前最困难的 Deno `__tsp_v` 机制已经可以被真正替代。

之后再做 `.tsp loader`、watcher、standalone 和业务层 port，风险会小得多。

---

# 33. 最终期望代码形态

TSP 的 request handler 最终不应该知道 module invalidation 如何实现。

目标：

```ts
async function handleTspRequest(filepath: string, context: PageContext) {
  const page = await Bun.TSP.loadPage(filepath);
  return await page(context);
}
```

Bun Fork 内：

```text
loadPage(path)
  -> canonicalize
  -> get PageSlot
  -> clean ? current : reload candidate
  -> targeted invalidate old affected graph keys
  -> read real filesystem
  -> resolve dependencies
  -> transpile TS/TSX/TSP
  -> instantiate/evaluate
  -> update graph edges
  -> atomic publish generation
  -> return page function
```

TSP TypeScript 层只负责：

```text
HTTP
context
DI
response/render
business services
```

Bun Fork 只负责：

```text
source loading
module identity
module graph
cache invalidation
generation
watcher signal
runtime compilation
```

这是最清晰、最容易长期维护的职责边界。

---

# 34. 结论

推荐采用 **“薄 Bun Fork + TSP 专用 Module Runtime”**，而不是 Bun `--hot` 或 TypeScript 层 cache bust。

改造的真正核心只有三个：

1. **同 canonical module key 的 targeted invalidation/re-instantiation；**
2. **reverse dependency graph + page-level generation；**
3. **compiled executable 中明确的 real-filesystem TSP source boundary。**

`.tsp -> TSX` 本身只是较小的 loader 改造。

当前 Deno 方案最复杂的部分——`dynamic-import-no-cache`、VFS bypass、`__tsp_v` generation token 及其跨依赖传播——在新架构中应该全部消失。

最重要的实施纪律是：

> **在 same-URL targeted module invalidation PoC 通过之前，不进行大规模 TSP 业务代码迁移。**

这能最大程度避免先投入大量迁移工作，最后才发现 Bun/JSC module registry 边界不符合预期。
