# TSP Native Runtime Refactor Plan

> Status: Proposed  
> Target branch: `codex/tsp-native-runtime`  
> Goal: 重构 TSP 运行时，逐步移除 Bun 依赖，建立由 TSP 自己拥有的编译、HTTP、路由、Worker、JavaScript Runtime 与运行时协议边界。  
> Primary language: Rust + JavaScriptCore + TypeScript/TSX

---

## 1. 背景

当前 TSP 正处于从 Bun-backed runtime 向 standalone native runtime 迁移的阶段。

现有 native workspace 已经开始拆分：

```text
native/crates/
├── tsp-cli
├── tsp-http
├── tsp-jsc
└── tsp-runtime
```

这个方向总体正确，但目前仍然存在几个结构性问题：

1. `tsp-runtime` 直接依赖 `tsp-jsc`，runtime domain 与具体 JavaScript 引擎实现耦合。
2. `tsp-runtime/src/worker.rs` 同时承担：
   - worker scheduling；
   - JavaScriptCore executor；
   - request dispatch protocol；
   - JavaScript bootstrap/prelude；
   - `tsp:*` built-in runtime；
   - Response serialization。
3. native build 仍直接依赖 Bun 维护的 WebKit/JSC build 和 `bun/vendor/mimalloc`。
4. 当前 JS 执行模型主要是：
   - `evaluate(source)`；
   - `drain_microtasks()`；
   - 再 `evaluate(JSON.stringify(...))`；
   尚未形成真正可扩展的 host async/event-loop 模型。
5. HTTP server、runtime domain、wire protocol、JS ABI 的边界还不稳定。
6. Worker 当前是 thread-based，而旧架构中很多可靠性能力来自 process isolation。
7. 文档中仍同时存在“Bun worker architecture”和“native JSC runtime architecture”，容易继续产生双重设计。
8. 当前实现更像 migration scaffold，而不是最终 production architecture。

因此，本计划的核心目标不是继续增加功能，而是先冻结边界、重新定义 ownership，然后按阶段迁移。

---

# 2. 总体目标

最终 TSP 应该成为一个不依赖 Bun runtime 的独立 server runtime。

目标结构：

```text
                       ┌─────────────────────┐
                       │      Compiler       │
                       │   TS/TSX -> JS      │
                       └──────────┬──────────┘
                                  │
                         manifest + bundle
                                  │
                                  ▼
┌────────────────────────────────────────────────────────┐
│                     Rust Master                        │
│                                                        │
│ HTTP -> Router -> Generation -> Admission -> Scheduler │
│                         │                              │
│              Sessions / Services / State               │
└─────────────────────────┬──────────────────────────────┘
                          │
                 Versioned IPC Protocol
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
      ┌───────────┐ ┌───────────┐ ┌───────────┐
      │ JSC Worker│ │ JSC Worker│ │ JSC Worker│
      │           │ │           │ │           │
      │ runtime.js│ │ runtime.js│ │ runtime.js│
      │ route code│ │ route code│ │ route code│
      └───────────┘ └───────────┘ └───────────┘
```

其中：

- Rust 拥有：
  - HTTP lifecycle；
  - routing；
  - generation registry；
  - worker lifecycle；
  - admission/backpressure；
  - deadlines/cancellation；
  - sessions；
  - durable services；
  - configuration；
  - diagnostics；
  - process management。

- JavaScript runtime 拥有：
  - `.tsp` application handler execution；
  - JSX rendering；
  - runtime built-ins；
  - request-scoped JavaScript values；
  - Promise execution；
  - application module execution。

- JavaScriptCore 只负责：
  - VM；
  - Realm/Global object；
  - JS values；
  - function call；
  - Promise job queue；
  - module/script evaluation；
  - host callback bridge。

- Bun 不再是 runtime architecture 的组成部分。

---

# 3. 非目标

本次重构阶段不优先解决以下问题：

- 极限 benchmark 优化；
- HTTP/3；
- 完整 Node.js compatibility；
- npm package runtime compatibility；
- client-side React；
- worker threads API；
- arbitrary native plugin ABI；
- inspector/devtools 完整支持；
- complex edge runtime sandbox；
- distributed runtime scheduling。

这些功能只有在 runtime contract 稳定后再讨论。

---

# 4. 核心设计原则

## 4.1 Runtime 不能依赖具体 JS engine

错误依赖：

```text
tsp-runtime
    ↓
tsp-jsc
```

目标依赖：

```text
tsp-runtime
    ↓
tsp-js

tsp-jsc
    ↓
tsp-js
```

`tsp-js` 定义 runtime 所需要的抽象能力。

`tsp-jsc` 只是一个 adapter。

即使未来永远只支持 JavaScriptCore，这个边界也必须成立。

---

## 4.2 JavaScriptCore 不是 Runtime

不能把：

```text
evaluate()
drain_microtasks()
```

当作 runtime abstraction。

JSC 只是 execution engine。

真正 runtime 还包括：

```text
module loader
request invocation
host functions
promise completion
event loop / reactor
timers
fetch
abort
service bridge
session bridge
generation lifecycle
```

---

## 4.3 Runtime JS ABI 必须独立版本化

`Response`、`Context`、`Request`、cookies、services、session、JSX 等不能继续作为 Rust 文件中的一大段字符串。

需要独立 runtime JS：

```text
runtime-js/
├── bootstrap.js
├── context.js
├── request.js
├── response.js
├── url.js
├── cookies.js
├── session.js
├── jsx.js
└── modules/
    ├── tsp-server.js
    └── tsp-html.js
```

构建后生成：

```text
tsp-runtime.js
```

并定义：

```text
TSP_RUNTIME_ABI_VERSION
```

---

## 4.4 Host 与 JS 之间只能通过显式协议通信

禁止：

- Rust domain struct 直接依赖 JS serializer 细节；
- JSC object pointer 跨 worker；
- native borrowed pointer 进入 page code；
- JS application value 长期保存在 host；
- runtime 靠任意 object shape 猜 response。

应该使用明确 envelope。

例如：

```json
{
  "version": 1,
  "requestId": "r_123",
  "method": "GET",
  "url": "/users/42",
  "headers": [],
  "body": {
    "kind": "bytes",
    "encoding": "base64",
    "data": ""
  }
}
```

响应：

```json
{
  "version": 1,
  "requestId": "r_123",
  "status": 200,
  "headers": [],
  "body": {
    "kind": "text",
    "data": "hello"
  },
  "effects": {
    "cookies": [],
    "session": []
  }
}
```

---

## 4.5 Durable state 必须归 Host

以下数据不能放在 disposable page generation 中：

- session backend；
- DB pool；
- logger；
- Redis connection；
- worker manager；
- service registry；
- application-wide cache；
- metrics registry。

Page generation 只拥有：

```text
compiled code
module graph
handler exports
page config
request-local state
```

---

## 4.6 Production worker 使用 process isolation

推荐目标：

```text
master process
    ├── worker process
    ├── worker process
    └── worker process
```

每个 worker：

```text
1 process
    ↓
1 JSC VM
```

原因：

- JS/native crash 不拖死 master；
- OOM 可隔离；
- timeout 可强杀；
- worker 可 recycle；
- memory limit 更容易做；
- hot reload generation 更清晰；
- future sandbox 更容易实现。

thread worker 可保留作为 test/dev backend，但不建议作为最终 production 默认。

---

# 5. 目标 Workspace

推荐最终 workspace：

```text
native/
├── Cargo.toml
│
├── crates/
│   ├── tsp-core/
│   │   ├── manifest.rs
│   │   ├── route.rs
│   │   ├── request.rs
│   │   ├── response.rs
│   │   ├── protocol.rs
│   │   ├── generation.rs
│   │   └── error.rs
│   │
│   ├── tsp-js/
│   │   ├── engine.rs
│   │   ├── runtime.rs
│   │   ├── module.rs
│   │   ├── value.rs
│   │   ├── promise.rs
│   │   └── error.rs
│   │
│   ├── tsp-jsc/
│   │   ├── ffi.rs
│   │   ├── engine.rs
│   │   ├── runtime.rs
│   │   ├── module.rs
│   │   ├── value.rs
│   │   ├── build.rs
│   │   ├── include/
│   │   └── cxx/
│   │
│   ├── tsp-runtime/
│   │   ├── host.rs
│   │   ├── scheduler.rs
│   │   ├── worker.rs
│   │   ├── generation_registry.rs
│   │   ├── admission.rs
│   │   ├── services.rs
│   │   ├── session.rs
│   │   └── lifecycle.rs
│   │
│   ├── tsp-worker/
│   │   ├── main.rs
│   │   ├── ipc.rs
│   │   └── executor.rs
│   │
│   ├── tsp-http/
│   │   ├── server.rs
│   │   ├── adapter.rs
│   │   └── limits.rs
│   │
│   └── tsp-cli/
│       ├── main.rs
│       ├── commands/
│       └── config.rs
│
├── runtime-js/
│   ├── src/
│   ├── tests/
│   └── dist/
│
├── fixtures/
└── tests/
```

---

# 6. Crate 职责

## 6.1 `tsp-core`

`tsp-core` 必须完全不依赖：

- JSC；
- Bun；
- HTTP server implementation；
- CLI；
- filesystem watcher；
- IPC implementation。

只包含稳定 domain model。

### 内容

```text
RouteSpec
CompiledManifest
RouteTable
RouteMatch
RequestEnvelope
ResponseEnvelope
BodyEnvelope
GenerationId
RequestId
RuntimeAbiVersion
ProtocolError
RouteError
```

### 验收标准

```bash
cargo tree -p tsp-core
```

不应出现：

```text
tsp-jsc
tsp-http
tokio
hyper
bun
webkit
```

---

## 6.2 `tsp-js`

定义 JS runtime 所需 capability。

初始接口不要设计得过大。

示例：

```rust
pub trait JsRuntime {
    type Error;

    fn initialize(&mut self, runtime_bundle: &[u8]) -> Result<(), Self::Error>;

    fn load_application(
        &mut self,
        generation: GenerationId,
        bundle: &[u8],
    ) -> Result<(), Self::Error>;

    fn start_request(
        &mut self,
        request: RequestEnvelope,
    ) -> Result<RequestExecution, Self::Error>;

    fn poll(
        &mut self,
        execution: &mut RequestExecution,
    ) -> Result<ExecutionPoll, Self::Error>;

    fn cancel(
        &mut self,
        execution: &mut RequestExecution,
    ) -> Result<(), Self::Error>;
}
```

关键点：

`runtime` 调用 `JsRuntime`，而不是调用 `JSC`.

---

## 6.3 `tsp-jsc`

负责：

```text
JSC initialization
VM ownership
thread affinity
GlobalObject
JSValue ownership
JSString conversion
function lookup
function call
Promise handling
microtask queue
native callback registration
native buffer ownership
exception conversion
module loading
```

禁止负责：

```text
routing
HTTP
session business logic
worker scheduling
request admission
application lifecycle
```

---

## 6.4 `tsp-runtime`

负责 Host runtime。

```text
generation registry
request lifecycle
worker lifecycle
scheduler
admission
timeouts
cancellation
service registry
session backend
reload
health
diagnostics
```

`tsp-runtime` 不允许出现：

```rust
use tsp_jsc::...
```

验收：

```bash
grep -R "tsp_jsc" native/crates/tsp-runtime
```

输出必须为空。

---

## 6.5 `tsp-worker`

负责 worker process。

生命周期：

```text
start
 ↓
initialize JSC
 ↓
load runtime JS
 ↓
announce READY
 ↓
load generation
 ↓
process requests
 ↓
recycle / shutdown
```

Worker IPC command：

```text
HELLO
LOAD_GENERATION
EXECUTE
CANCEL
PING
SHUTDOWN
```

Worker response：

```text
READY
GENERATION_READY
RESULT
ERROR
PONG
EXITING
```

---

## 6.6 `tsp-http`

只做：

```text
HTTP transport
    ↓
tsp-core::RequestEnvelope
```

以及：

```text
tsp-core::ResponseEnvelope
    ↓
HTTP response
```

不要在这里做：

- routing；
- session；
- JS execution；
- generation lookup。

---

## 6.7 `tsp-cli`

只作为 composition root。

负责：

```text
load config
load manifest
initialize runtime
start server
spawn workers
wire dependencies
handle signals
```

业务逻辑不能继续堆在 `main.rs`。

---

# 7. JavaScript Runtime 设计

## 7.1 Bootstrap

VM 初始化只执行一次：

```text
runtime.js
```

runtime.js 提供：

```text
__tsp.bootstrap()
__tsp.loadGeneration()
__tsp.dispatch()
__tsp.cancel()
```

Rust 不再每个 request `format!()` 一段 JavaScript。

---

## 7.2 Request Dispatch

错误模型：

```text
Rust
  ↓
format JS source
  ↓
JSC parse
  ↓
execute
```

目标：

```text
Rust
  ↓
create JS request value
  ↓
call cached __tsp.dispatch function
  ↓
Promise / Response
```

如果 FFI 初期不方便传 object，可先使用 JSON string。

但必须保证：

```text
request path
method
params
body
headers
```

不是通过 source-code interpolation 进入 JS。

---

## 7.3 Response Protocol

必须定义：

```rust
pub enum BodyEnvelope {
    Empty,
    Text(String),
    Bytes(Vec<u8>),
}
```

未来可扩：

```rust
Stream(...)
File(...)
```

禁止再让：

```text
JS body: string
Rust body: Vec<u8>
```

依赖 serde 隐式转换。

---

## 7.4 Async Execution

当前：

```text
handler
 ↓
drain_microtasks()
 ↓
如果 pending => error
```

仅支持纯 microtask Promise。

目标：

```text
start handler
    ↓
drain JS jobs
    ↓
check pending host operations
    ↓
poll reactor
    ↓
complete native promise
    ↓
drain JS jobs
    ↓
until resolved/rejected/deadline
```

第一阶段支持：

```text
Promise
queueMicrotask
native async callback
timer
abort
```

第二阶段支持：

```text
fetch
session backend
service calls
```

---

# 8. Event Loop / Reactor

不要复制 Bun event loop。

TSP 只实现 server runtime 所需最小 reactor。

建议：

```text
WorkerRuntime
├── JsRuntime
├── HostTaskQueue
├── Timers
├── PendingOperations
└── CancellationRegistry
```

执行循环：

```rust
loop {
    js.drain_jobs()?;

    if execution.is_finished() {
        break;
    }

    if deadline.expired() {
        cancel();
        break;
    }

    reactor.poll(timeout)?;

    complete_ready_host_operations();

    if cancelled {
        reject_abort_signal();
    }
}
```

---

# 9. Built-in Modules

保留：

```text
tsp:server
tsp:html
```

未来可以增加：

```text
tsp:session
tsp:services
```

但 built-in module 必须通过 runtime module registry 注册。

禁止依赖：

```text
globalThis.__tsp_builtin_modules
```

作为长期模块系统。

初期可以作为 bootstrap implementation detail，但 API 需要抽象。

---

# 10. Compiler Contract

Compiler 与 runtime 通过 artifact communication。

Compiler 输出：

```text
dist/
├── manifest.json
├── bundle.js
└── metadata.json
```

`manifest.json`：

```json
{
  "version": 1,
  "runtimeAbi": 1,
  "compiler": "tspc",
  "routes": [],
  "modules": []
}
```

runtime startup 必须验证：

```text
manifest version
runtime ABI
compiler compatibility
route shape
duplicate route
output existence
```

---

# 11. Generation Model

必须保留旧设计中非常重要的 generation concept。

```text
source change
    ↓
compile candidate
    ↓
validate
    ↓
load worker candidate
    ↓
worker ACK
    ↓
publish atomically
```

Request 必须 pin：

```text
Request
    ↓
GenerationId
```

请求结束之前 generation 不可被释放。

失败 reload：

```text
new generation failed
        ↓
old generation continues
```

---

# 12. Worker Model

## 12.1 Production

默认：

```text
process workers
```

参数：

```text
worker_count
max_requests_per_worker
max_worker_age
max_memory
request_timeout
startup_timeout
shutdown_timeout
queue_capacity
```

---

## 12.2 Admission

禁止 unbounded channel。

必须使用 bounded admission。

例如：

```text
HTTP
 ↓
admission semaphore
 ↓
scheduler
 ↓
bounded worker queue
```

超载：

```text
503 Service Unavailable
```

或 configurable：

```text
429 Too Many Requests
```

---

## 12.3 Scheduling

第一阶段：

```text
least-loaded worker
```

不要 round-robin + unbounded queue。

Worker 状态：

```rust
struct WorkerState {
    inflight: usize,
    queued: usize,
    generation: GenerationId,
    health: Health,
}
```

---

## 12.4 Worker Recycling

触发条件：

```text
request count
age
memory
fatal execution error
generation incompatibility
health failure
```

流程：

```text
spawn replacement
 ↓
load generation
 ↓
READY
 ↓
stop admitting old worker
 ↓
wait inflight
 ↓
terminate old worker
```

---

# 13. IPC

第一阶段推荐简单 binary framing。

例如：

```text
u32 length
u8 message type
payload
```

payload 初期可用 JSON。

不要一开始就设计复杂 binary protocol。

目标：

```text
simple
versioned
observable
debuggable
bounded
```

IPC 不允许传：

- native pointer；
- JSC object；
- process-local allocator buffer。

---

# 14. HTTP Layer

当前自写 HTTP/1.1 parser 可以继续作为 prototype，但不建议长期承担 production wire protocol。

推荐目标：

```text
battle-tested Rust HTTP stack
        ↓
tsp-http adapter
        ↓
tsp-core RequestEnvelope
```

必须至少支持：

```text
keep-alive
chunked request
chunked response
body limits
header limits
timeouts
connection limits
graceful shutdown
```

TSP 不应该把 HTTP parser correctness 当作核心研发方向。

---

# 15. 去 Bun 路线

## Phase A — Runtime 不调用 Bun

当前阶段基本已经开始实现。

验收：

```text
production tspserver 不 spawn bun
route 不由 bun worker 执行
HTTP 不依赖 Bun server
```

---

## Phase B — Rust runtime 不依赖 Bun types

验收：

```bash
grep -R "bun_" native/crates
grep -R "Bun" native/crates
```

只有 migration 注释允许临时存在。

---

## Phase C — Build 不读取 Bun source tree

必须删除：

```text
bun/vendor/mimalloc
```

及任何：

```text
../../../bun/...
```

验收：

```bash
grep -R "../.*bun" native
```

为空。

---

## Phase D — 独立 JSC SDK

建立：

```text
TSP JSC SDK
├── include/
├── lib/
├── licenses/
└── metadata.json
```

SDK artifact 必须包含所有 ABI-compatible native dependencies。

例如：

```text
JavaScriptCore
WTF
bmalloc / allocator
ICU
platform dependencies
```

不能由 Cargo 临时从 Bun source 拼装。

---

## Phase E — CI 不 checkout/build Bun

最终 CI：

```text
checkout tsp
download tsp-jsc-sdk
cargo build
run tests
package
```

而不是：

```text
checkout bun
build bun dependencies
extract webkit
build tsp
```

---

# 16. JSC SDK Strategy

推荐将 JavaScriptCore 当成 TSP 自己维护的 toolchain dependency。

需要：

```text
jsc-sdk-version
webkit commit
build flags
compiler version
target triple
allocator
ICU version
ABI metadata
```

示例：

```json
{
  "sdkVersion": 1,
  "webkitCommit": "...",
  "target": "x86_64-unknown-linux-gnu",
  "allocator": "mimalloc",
  "runtimeAbi": 1
}
```

Rust build 必须验证 SDK metadata。

---

# 17. Error Model

统一 error family。

建议：

```text
TSP1xxx compiler/manifest/route
TSP2xxx HTTP/request
TSP3xxx runtime/worker
TSP4xxx JS/JSC
TSP5xxx services/session
```

错误必须具有：

```rust
code
kind
message
source
request_id
worker_id
generation_id
```

production response 不暴露：

```text
filesystem path
native stack
JSC pointer
secret
environment
internal ABI
```

---

# 18. Observability

最少记录：

```text
request_id
route
generation
worker_id
queue_wait
execution_time
response_time
status
worker_restart_reason
reload_status
```

metrics：

```text
requests_total
requests_inflight
requests_queued
worker_count
worker_restarts_total
worker_execution_seconds
generation_reload_total
generation_reload_failed_total
```

---

# 19. Configuration

配置统一由 host 读取。

页面不得直接访问完整：

```text
process.env
```

建议：

```text
config.jsonc
environment
CLI flags
```

解析后产生：

```rust
RuntimeConfig
```

再把允许暴露给 page 的 subset 显式传入。

---

# 20. Security Boundaries

必须明确：

```text
filesystem root
public root
route source root
import root
request body limit
header limit
service permission
environment exposure
worker memory
worker timeout
```

禁止 route import：

```text
outside application root
arbitrary native dynamic library
another .tsp route as reusable module
```

---

# 21. 测试策略

每层单独测试。

## 21.1 `tsp-core`

```text
route precedence
duplicate route
catch-all
manifest parsing
protocol serialization
body envelope
ABI version
```

---

## 21.2 `tsp-jsc`

```text
VM ownership
wrong-thread access
JS exception
native buffer ownership
function call
promise resolve
promise reject
microtask
host callback
runtime initialization
```

---

## 21.3 `tsp-runtime`

使用 mock `JsRuntime`.

测试：

```text
bounded admission
timeout
cancellation
scheduler
generation publish
last-known-good
worker recycle
service ownership
```

---

## 21.4 Worker Integration

真正启动：

```text
tsp-worker
```

测试：

```text
READY handshake
LOAD_GENERATION
EXECUTE
CANCEL
crash replacement
timeout
shutdown
```

---

## 21.5 Native E2E

必须覆盖：

```text
GET
POST
dynamic route
catch-all
query
cookie
JSON
HTML
JSX
async handler
handler error
404
405
reload
worker crash
timeout
```

---

# 22. CI Matrix

最低：

```text
Linux x64
Windows x64
macOS arm64
```

Pipeline：

```text
cargo fmt
cargo clippy
cargo test core
cargo test runtime mock
cargo test jsc
worker integration
native e2e
release build
package smoke
```

JSC SDK cache 独立管理。

---

# 23. Migration Phases

---

## Phase 0 — Freeze

目标：停止继续在 migration architecture 上增加 runtime feature。

任务：

- freeze `worker.rs` feature growth；
- freeze Bun-specific native integration；
- 新 feature 只能进入新边界。

完成条件：

```text
Architecture ADR merged
workspace target layout agreed
```

---

## Phase 1 — Extract `tsp-core`

任务：

从 `tsp-runtime` / `tsp-http` 抽：

```text
RouteSpec
CompiledManifest
RouteTable
RequestEnvelope
ResponseEnvelope
BodyEnvelope
Protocol version
```

完成条件：

```text
tsp-core no JSC
tsp-core no HTTP implementation
all route tests migrated
```

---

## Phase 2 — Introduce `tsp-js`

任务：

建立 engine/runtime abstraction。

将 runtime 对 JSC 的直接依赖移除。

完成条件：

```bash
grep -R "tsp_jsc" native/crates/tsp-runtime
```

为空。

---

## Phase 3 — Extract Runtime JS

任务：

把 `RUNTIME_PRELUDE` 移到：

```text
native/runtime-js
```

建立独立测试和 bundle。

完成条件：

```text
worker.rs 不包含大段 JS source
runtime ABI 有版本号
JS runtime 有独立 unit tests
```

---

## Phase 4 — Fix Request/Response Protocol

任务：

定义稳定 envelope。

完成条件：

```text
text response
binary response
headers
cookies
error
```

全部 round-trip 测试通过。

---

## Phase 5 — Replace Per-request Eval

任务：

VM 初始化时缓存：

```text
dispatch function
```

request hot path 不再：

```text
format JS source
evaluate request script
```

完成条件：

```text
request dispatch 使用 function call
source injection path 为 0
```

---

## Phase 6 — Async Runtime

任务：

增加：

```text
runtime poll loop
pending operation registry
host async completion
timeout
abort
```

完成条件：

```ts
await nativeAsyncOperation()
```

可以真正 suspend/resume。

---

## Phase 7 — Process Worker

任务：

建立：

```text
tsp-worker
IPC
worker manager
health
restart
bounded queues
```

完成条件：

```text
worker crash 不影响 master
worker timeout 可 kill
worker replacement automatic
```

---

## Phase 8 — Generation Registry

任务：

把旧 runtime 的 disposable generation concept 迁移到 native runtime。

完成条件：

```text
atomic publish
request pin
last-known-good
worker generation load
```

---

## Phase 9 — Remove Bun Build Dependency

任务：

建立 independent JSC SDK。

删除：

```text
bun/vendor
Bun WebKit builder dependency
```

完成条件：

```bash
grep -R "bun/" native
```

为空。

CI 不需要 Bun repo。

---

## Phase 10 — Production Hardening

任务：

```text
HTTP keepalive
bounded concurrency
graceful shutdown
resource limits
metrics
diagnostics
worker recycling
package
cross-platform CI
```

---

# 24. Definition of Done

整体重构完成必须满足：

## Architecture

- `tsp-runtime` 不依赖 `tsp-jsc`；
- `tsp-core` 不依赖 engine/HTTP；
- `tsp-jsc` 不知道 route/session/http；
- `tsp-http` 不知道 JSC；
- `tsp-cli` 只做 composition。

## Bun

```bash
grep -R "bun/" native
```

无运行或构建依赖。

release package 不包含 Bun runtime。

CI 构建 TSP 不要求 Bun checkout。

## Runtime

- request hot path 无 JS source interpolation；
- async handler 可真正 suspend/resume；
- timeout/cancel 有明确语义；
- worker queue bounded；
- worker crash 可恢复；
- generation 可原子切换。

## Protocol

- request protocol versioned；
- response protocol versioned；
- text/binary body 明确；
- cookie/session effects 显式；
- error envelope 明确。

## Reliability

- worker OOM/crash 不拖死 master；
- failed reload 保留 last-known-good；
- shutdown graceful；
- overload 有 backpressure。

---

# 25. 建议的 ADR

建议新增：

```text
docs/reference/adr/
├── 0010-runtime-v2-architecture.md
├── 0011-javascript-engine-boundary.md
├── 0012-runtime-js-abi.md
├── 0013-worker-process-isolation.md
├── 0014-host-js-protocol.md
├── 0015-jsc-sdk.md
└── 0016-generation-lifecycle.md
```

---

# 26. 首批具体 PR 拆分

建议不要一个大 PR 完成。

## PR 1

```text
refactor: introduce tsp-core
```

只迁：

```text
manifest
route
protocol base types
```

---

## PR 2

```text
refactor: introduce tsp-js engine boundary
```

增加 trait。

runtime test 使用 mock runtime。

---

## PR 3

```text
refactor: move JSC implementation behind tsp-js
```

保证：

```text
tsp-runtime -> tsp-js
tsp-jsc -> tsp-js
```

---

## PR 4

```text
refactor: extract runtime-js bundle
```

移除 `RUNTIME_PRELUDE`.

---

## PR 5

```text
fix: define request and response envelopes
```

同时修复 string body / `Vec<u8>` contract。

---

## PR 6

```text
perf: replace request eval with cached dispatch function
```

---

## PR 7

```text
runtime: add async execution loop
```

---

## PR 8

```text
runtime: introduce worker process protocol
```

---

## PR 9

```text
runtime: add generation registry
```

---

## PR 10

```text
build: replace Bun WebKit dependency with TSP JSC SDK
```

---

# 27. 当前最优先问题

按优先级：

## P0

1. `tsp-runtime -> tsp-jsc` 耦合；
2. Bun WebKit/mimalloc build dependency；
3. request/response protocol 不稳定；
4. per-request JS evaluate；
5. async execution model 不完整。

## P1

6. worker unbounded queue；
7. thread worker 缺乏 crash isolation；
8. runtime JS 写在 Rust source；
9. generation architecture 尚未迁移；
10. 文档 architecture 冲突。

## P2

11. HTTP production hardening；
12. observability；
13. worker recycling；
14. package/CI；
15. performance tuning。

---

# 28. 最终架构约束

以后任何代码 review 都可以用这些规则检查：

```text
Rule 1:
Domain 不知道 engine。

Rule 2:
Engine 不知道 HTTP。

Rule 3:
HTTP 不知道 JS。

Rule 4:
Application JS 不持有 native durable state。

Rule 5:
Host 和 Worker 不共享 native object。

Rule 6:
Host 和 JS 之间只有 versioned protocol。

Rule 7:
Worker queue 必须 bounded。

Rule 8:
Worker failure 必须 isolated。

Rule 9:
Generation 必须 immutable + atomic publish。

Rule 10:
Bun 不能作为 hidden build/runtime dependency。
```

---

# 29. 推荐最终依赖图

```text
                         tsp-cli
                            │
                            ▼
                       tsp-runtime
                    ┌───────┼────────┐
                    │       │        │
                    ▼       ▼        ▼
               tsp-core  tsp-http  worker-manager
                    ▲
                    │
                 tsp-js
                    ▲
                    │
                 tsp-jsc
                    │
                    ▼
              TSP JSC SDK
                    │
                    ▼
               JavaScriptCore
```

Worker：

```text
tsp-worker
   │
   ├── tsp-core
   ├── tsp-js
   └── tsp-jsc
```

Runtime JS：

```text
runtime-js
   ↓
tsp-runtime.js
   ↓
loaded into each worker VM
```

---

# 30. 结论

当前 `codex/tsp-native-runtime` 分支是一个合理的 migration prototype，但不应直接作为最终 architecture 继续堆功能。

当前正确的工作方向不是继续扩大：

```text
tsp-runtime + tsp-jsc + worker.rs
```

而是尽快完成四个关键切割：

```text
1. Domain 与 JSC 切割
2. Runtime JS 与 Rust worker 切割
3. Host/JS protocol 与 HTTP struct 切割
4. TSP build 与 Bun source tree 切割
```

完成这些边界之后，TSP 才真正拥有自己的 runtime architecture。

最终设计目标不是“重写一个 Bun”，而是：

> 构建一个只实现 TSP 所需能力的、小而明确的 server-side JavaScript runtime。

JavaScriptCore 提供执行引擎。

Rust 提供 Host、生命周期、资源、调度与隔离。

TSP runtime JS 提供 application ABI。

Compiler 提供 immutable application artifact。

四层各自稳定，才是这次重构真正完成的标志。
