# TSP 完整重构计划与技术规格

> Status: Draft / Architecture Proposal  
> Target: TSP
> Compatibility: **不兼容 legacy TSP**
> Date: 2026-08-24

---

## 0. 文档目的

本文定义 TSP 的目标架构、运行时边界、`.tsp` 文件规范、Rust/JSC 交互模型、路由、请求上下文、响应模型、JSX runtime、依赖图、热更新、持久资源、构建发布和测试标准。

TSP 不以迁移 legacy TSP 为主要约束，而以重新建立一个长期稳定、实现边界清晰、运行时状态可控的 Server Page Runtime 为目标。

本文中的关键设计原则优先级高于现有实现。除非后续明确修改规范，否则实现应服从本文协议，而不是反向迁就已有代码。

---

# 1. 产品定义

TSP 定义为：

> **一个以 Rust 为 Host Runtime、以 JavaScriptCore 为 TypeScript/JavaScript 执行虚拟机、以 `.tsp` 作为 Server Page Entry Module 的 Web Runtime。**

TSP 不是 React Framework，也不是 Node/Bun application framework 的薄封装。

目标结构：

```text
TSP
│
├── Native Host Runtime (Rust)
│   ├── process / CLI
│   ├── config
│   ├── HTTP server
│   ├── router
│   ├── path security
│   ├── static files
│   ├── request parsing
│   ├── cookies
│   ├── sessions
│   ├── logging
│   ├── service registry
│   ├── module graph
│   ├── file watcher
│   ├── generation manager
│   ├── reload / LKG
│   └── JSC bridge
│
├── JavaScriptCore VM
│   ├── .tsp page execution
│   ├── .ts/.tsx application modules
│   ├── npm compatibility modules when needed
│   └── minimal tsp:* runtime bindings
│
└── Application
    ├── pages/**/*.tsp
    ├── components/**/*.tsx
    ├── lib/**/*.ts
    └── tsp.config.*
```

核心关系：

```text
Rust owns lifecycle.
JSC executes application code.
.tsp defines page behavior.
```

---

# 2. current 核心目标

## 2.1 必须实现

1. `tspserver` 是原生入口，不依赖 `src/main.ts`。
2. `.tsp` 保持合法 TypeScript/TSX，不发明新的模板语法。
3. `.tsp` 是 route entry point，而不是普通可复用模块。
4. HTTP、routing、static、session、logging、module graph、reload 等框架生命周期归 Rust。
5. JavaScriptCore 只执行页面代码和应用 JavaScript/TypeScript。
6. 页面代码可使用 TS/TSX、ESM、async/await。
7. 支持文件系统路由。
8. 支持精确依赖图和基于 Page Root 的定向热更新。
9. 支持原子 generation 发布。
10. 编译失败时支持 Last Known Good。
11. 老请求可以继续使用旧 generation，新请求切换到新 generation。
12. Runtime 持久资源不因为 page reload 而重建。
13. 生产模式支持单可执行文件 + 外部可修改 application 目录。
14. 不要求安装完整 Bun CLI 才能运行已发布的 `tspserver`。
15. `.tsp` 文件协议必须可以长期稳定演进。

## 2.2 明确不做

TSP 第一阶段不追求：

- legacy TSP 兼容层；
- 自动迁移旧 `.tsp`；
- React 兼容；
- React Server Components；
- Next.js 式复杂 layout/intercept/parallel route；
- 客户端 hydration framework；
- 内置 SPA router；
- 自定义 TypeScript compiler；
- 自定义模板语言；
- 完整 Node.js server framework 兼容；
- 为了“纯 Rust”重写所有 npm 库。

---

# 3. 最高优先级设计原则

## 3.1 `.tsp` 必须始终是标准 TSX

`.tsp` 文件必须能够通过标准 TypeScript/JSX parser 解析。

禁止增加如下自定义语法：

```text
<template>
<script>
{% %}
@page
@fragment
```

允许增加的是 **module semantics**，不是 language syntax。

因此：

```text
.tsp syntax == TSX syntax
.tsp semantics != ordinary .tsx module semantics
```

## 3.2 `.tsp` 是生命周期边界

`.tsp` 是 Route Root / Page Root。

它的职责是：

- 声明 HTTP handlers；
- 声明 page metadata；
- 声明 fragments；
- import 普通 `.ts/.tsx` 依赖；
- 返回 HTML tree 或 Response。

`.tsp` 不作为通用 library module 使用。

## 3.3 长生命周期状态归 Native Runtime

以下状态默认归 Rust runtime：

- config；
- router；
- session store；
- DB pool handles；
- Redis connection/pool；
- logger；
- service registry；
- watcher；
- module graph；
- page registry；
- reload generations；
- cache metadata；
- runtime metrics。

页面模块必须被视为 disposable。

## 3.4 JS module generation 不拥有系统资源

应用代码可以拿到资源代理，但不能成为资源真正 owner。

例如：

```text
Page generation 17
  -> ctx.services.db
      -> Native ServiceHandle(42)
          -> Persistent DB Pool
```

当 generation 17 被回收时，DB Pool 不被销毁。

## 3.5 JSX 不等于 React

TSP 默认设计自己的 Server JSX runtime。

JSX 是语法；React 是其中一种 runtime。

TSP 不把 React Element 作为核心 ABI。

---

# 4. `.tsp` 文件规范

本章属于 **TSP Language/Runtime Contract**，优先冻结。

## 4.1 基本格式

一个最简单的页面：

```tsx
import type { Context } from "tsp:server";

export function GET(ctx: Context) {
  return <h1>Hello TSP</h1>;
}
```

该文件：

```text
pages/index.tsp
```

映射：

```text
GET /
```

## 4.2 合法 exports

TSP 第一版保留以下框架级 exports：

```ts
export const config = ...;

export function GET(ctx) {}
export function POST(ctx) {}
export function PUT(ctx) {}
export function PATCH(ctx) {}
export function DELETE(ctx) {}
export function HEAD(ctx) {}
export function OPTIONS(ctx) {}

export const someFragment = fragment(...);
```

其他普通 exports 允许存在，但只作为模块内部/测试用途，不自动成为 HTTP endpoint。

### 4.2.1 不使用 default export

不推荐也不支持把 default export 解释为默认 page handler：

```tsx
// current: 不作为 page handler
export default function Page() {}
```

理由：

- HTTP method 语义不清晰；
- loader 需要额外 magic；
- named export 可以在模块扫描阶段直接提取 metadata；
- 更适合 API + HTML 混合页面。

## 4.3 Handler 类型

规范类型：

```ts
type PageHandler = (
  ctx: Context
) => HandlerResult | Promise<HandlerResult>;
```

`HandlerResult`：

```ts
type HandlerResult =
  | HtmlNode
  | Response;
```

第一版不引入：

```ts
string
number
object
{ redirect: ... }
{ json: ... }
```

作为隐式响应协议。

所有响应必须明确变成 `HtmlNode` 或 `Response`。

## 4.4 页面示例

```tsx
import {
  type Context,
  type PageConfig,
  json,
  redirect,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const users = await ctx.services.db.users.list();

  return (
    <html>
      <head>
        <title>Users</title>
      </head>
      <body>
        <h1>Users</h1>
        <UserList users={users} />
      </body>
    </html>
  );
}

export async function POST(ctx: Context) {
  const form = await ctx.request.formData();

  await ctx.services.db.users.create({
    name: String(form.get("name")),
  });

  return redirect("/users");
}

function UserList({ users }: { users: User[] }) {
  return (
    <ul>
      {users.map(user => <li>{user.name}</li>)}
    </ul>
  );
}
```

---

# 5. `.tsp` import 规则

## 5.1 `.tsp` 不允许被 application import

禁止：

```tsx
import UsersPage from "./users.tsp";
```

默认报 compile/load error：

```text
TSP2003: .tsp modules are route entry modules and cannot be imported.
Move reusable code to .ts or .tsx.
```

理由：

- `.tsp` 是 generation root；
- 避免 page root 之间产生生命周期耦合；
- 简化 reverse dependency graph；
- 简化 route ownership；
- 避免两个 PageSlot 共享同一个 route module instance。

## 5.2 `.tsp` 可以 import

允许：

```text
.ts
.tsx
.js
.jsx
.json
npm package
tsp:* builtin
node:* / Bun supported runtime builtin（按兼容策略）
```

## 5.3 application source graph 与 external graph

定义两类模块：

### Reloadable Application Module

满足：

```text
canonical_path under application.root
AND supported source extension
```

例如：

```text
pages/users/[id].tsp
components/UserCard.tsx
lib/format.ts
```

这些进入 TSP ModuleGraph。

### Persistent External Module

例如：

```text
node_modules/zod/...
tsp:server
tsp:jsx-runtime
node:crypto
```

默认不进入 Page Root reload graph，或者只记录外部边界，不监听文件变更。

---

# 6. 文件系统路由规范

## 6.1 默认目录

```text
pages/
```

可通过配置改变：

```toml
[routes]
dir = "pages"
```

## 6.2 基础映射

```text
pages/index.tsp             /
pages/login.tsp             /login
pages/users/index.tsp       /users
pages/users/new.tsp         /users/new
pages/users/[id].tsp        /users/:id
pages/posts/[slug].tsp      /posts/:slug
```

## 6.3 Dynamic segment

语法：

```text
[id]
[slug]
```

要求 segment name：

```regex
[A-Za-z_][A-Za-z0-9_]*
```

提供：

```ts
ctx.params.id
ctx.params.slug
```

## 6.4 Catch-all

current contract 初版建议只支持一种：

```text
[...path].tsp
```

匹配：

```text
/files/a/b/c
```

提供：

```ts
ctx.params.path === "a/b/c"
```

不在 current contract 首版支持 optional catch-all。

## 6.5 路由优先级

静态优先于动态，动态优先于 catch-all：

```text
/users/new
/users/[id]
/users/[...path]
```

编译 route tree 时检测歧义。

例如：

```text
pages/users/[id].tsp
pages/users/[name].tsp
```

属于冲突，应启动失败：

```text
TSP1004: ambiguous routes /users/[id] and /users/[name]
```

## 6.6 Route table 启动时建立

Rust runtime 启动时：

```text
scan pages/
 -> validate filenames
 -> canonicalize
 -> build radix/tree matcher
 -> allocate PageSlot
 -> extract static page metadata when possible
```

生产请求阶段不扫描文件系统。

---

# 7. Page Config 规范

建议：

```ts
export interface PageConfig {
  auth?: "none" | "optional" | "required";

  cache?:
    | "no-store"
    | "private"
    | "public";

  bodyLimit?: number;

  timeoutMs?: number;

  methods?: readonly HttpMethod[];
}
```

页面：

```tsx
export const config = {
  auth: "required",
  bodyLimit: 2 * 1024 * 1024,
} satisfies PageConfig;
```

### 原则

`config` 应尽量静态可分析。

禁止依赖请求时动态计算：

```tsx
export const config = await fetch(...); // 禁止
```

建议 loader 规定：

- `config` 可以是 plain object；
- 不执行任意 async 初始化来获得 route metadata；
- 无法静态提取时在 module evaluate 后读取，但不能影响 route identity。

---

# 8. Context 规范

`Context` 是页面调用的核心 ABI。

建议第一版：

```ts
export interface Context<S = Services> {
  readonly request: TspRequest;
  readonly url: URL;

  readonly method: string;
  readonly params: Readonly<Record<string, string>>;
  readonly query: URLSearchParams;

  readonly cookies: Cookies;
  readonly session: Session;

  readonly services: S;

  readonly signal: AbortSignal;

  readonly route: RouteInfo;

  fragment(name: string, params?: Record<string, string>): string;
}
```

## 8.1 Context 每请求新建

```text
Request A -> Context A
Request B -> Context B
```

Context 不能跨请求缓存。

## 8.2 Context native ownership

Rust 负责 RequestContext 生命周期。

JS 看到的是 wrapper/proxy。

建议：

```text
Rust RequestContext
  owns native request state
    ↓
JSC ContextObject
  contains opaque native handle
```

避免将完整请求数据复制到 JS。

## 8.3 Context 在 handler 返回后失效

handler 完成后，context native handle 进入 closed 状态。

后续访问：

```text
TSP RuntimeError: Request context is no longer active
```

这样可以避免用户偷偷将 `ctx` 放入全局变量造成生命周期泄漏。

---

# 9. Request API

尽量遵循 Web Request 语义，但不要求内部实现就是 JS Request。

建议：

```ts
interface TspRequest {
  readonly method: string;
  readonly headers: Headers;
  readonly url: string;

  text(): Promise<string>;
  json<T = unknown>(): Promise<T>;
  formData(): Promise<FormData>;
  arrayBuffer(): Promise<ArrayBuffer>;
}
```

## 9.1 body 只能消费一次

行为对齐 Web Request：

```ts
await ctx.request.json();
await ctx.request.text(); // error: body already consumed
```

内部 Rust body stream 只保持一个 owner。

## 9.2 Body 限制

全局：

```toml
[http]
max_body_size = "10mb"
```

页面可降低：

```ts
export const config = {
  bodyLimit: 1024 * 1024,
};
```

页面不能超过 global hard limit。

## 9.3 multipart

multipart parser 建议 native 实现。

大文件不得强制全部进入 JS heap。

推荐：

```ts
interface UploadedFile {
  readonly name: string;
  readonly type: string;
  readonly size: number;
  readonly tempPath?: string;

  arrayBuffer(): Promise<ArrayBuffer>;
  stream(): ReadableStream<Uint8Array>;
}
```

后续可以增加 native zero-copy file handling。

---

# 10. Response 规范

TSP 只允许显式结果。

## 10.1 HTML

```tsx
return <h1>Hello</h1>;
```

自动转换成：

```text
status = 200
content-type = text/html; charset=utf-8
body = rendered HtmlNode
```

## 10.2 标准 Response

```ts
return new Response("ok", {
  status: 201,
});
```

## 10.3 helper

`tsp:server` 提供：

```ts
json(value, init?)
redirect(location, status?)
text(value, init?)
html(node, init?)
notFound()
```

例如：

```ts
return json({ ok: true });
```

```ts
return redirect("/login");
```

## 10.4 禁止 shape magic

不支持：

```ts
return { redirect: "/login" };
return { status: 404, body: "x" };
```

普通对象作为返回值属于 runtime type error：

```text
TSP3001: handler returned unsupported value Object.
Expected HtmlNode or Response.
```

---

# 11. TSP JSX Runtime

这是 current 的重要技术方向。

## 11.1 目标

不依赖 React / react-dom/server。

JSX 编译到：

```ts
import {
  jsx,
  jsxs,
  Fragment,
} from "tsp:jsx-runtime";
```

## 11.2 TypeScript 配置

概念配置：

```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "tsp"
  }
}
```

最终 resolver 将：

```text
tsp/jsx-runtime
```

映射到 native/runtime builtin。

## 11.3 HtmlNode ABI

推荐 JS 侧不暴露复杂 class hierarchy。

逻辑模型：

```rust
enum HtmlNode {
    Empty,
    Text(TextValue),
    Element(ElementNode),
    Fragment(Vec<NodeRef>),
    Async(AsyncNodeHandle),
    Raw(TrustedHtmlHandle),
}
```

但实际 JSC bridge 应尽量避免将整个树复制成 Rust enum。

第一版可采用 opaque JSC objects + native renderer walker。

## 11.4 JSX children 规则

必须在规范中确定：

### 输出为空

```tsx
{null}
{undefined}
{false}
{true}
```

均输出空。

### 数值

```tsx
{42}
```

输出：

```text
42
```

### 字符串

必须 HTML escape：

```tsx
{"<script>"}
```

输出：

```html
&lt;script&gt;
```

### Array

递归 flatten：

```tsx
{[<a />, <b />]}
```

### Unsupported object

普通 object 不允许直接渲染。

抛：

```text
TSP3102: object cannot be rendered as an HTML child
```

## 11.5 Attribute 规则

规范建议：

```tsx
<div class="a" />
<label for="x" />
```

TSP server JSX 不必继承 React 的 `className/htmlFor` 历史语义。

为降低学习成本，可以同时允许：

```text
class
className
for
htmlFor
```

但建议 canonical output 使用 HTML 原生名称。

### boolean attributes

```tsx
<input disabled={true} />
```

输出：

```html
<input disabled>
```

`false/null/undefined` 不输出。

### event handlers

默认禁止服务器 JSX：

```tsx
<button onClick={fn}>...</button>
```

因为服务端无法序列化函数。

compile/runtime 报错：

```text
TSP3105: function-valued HTML attributes are not serializable
```

如果将来做 client runtime，再通过明确机制添加。

## 11.6 Raw HTML

禁止 React 风格 magic property。

提供显式 API：

```tsx
import { raw } from "tsp:html";

<div>{raw(trustedHtml)}</div>
```

`raw()` 名字本身应表达危险性。

可以进一步设计：

```ts
TrustedHtml
```

brand type，避免普通字符串误传。

---

# 12. Component 语义

## 12.1 普通 function component

```tsx
function Greeting({ name }: { name: string }) {
  return <strong>Hello {name}</strong>;
}
```

## 12.2 Async component

TSP 推荐原生支持：

```tsx
async function UserName({ id }: { id: number }) {
  const user = await db.users.get(id);
  return <span>{user.name}</span>;
}
```

JSX renderer 接受 Promise / AsyncNode。

## 12.3 Component 不维护客户端状态

TSP JSX component 是 server render function，不定义 React hook 模型。

不存在：

```text
useState
useEffect
useLayoutEffect
```

这使 server runtime 语义保持简单。

## 12.4 Error boundary

current contract 可以暂不实现 component-level error boundary。

错误由 page handler / route error handler 捕获。

未来可以增加：

```tsx
<Boundary fallback={...}>
```

但不应阻塞首版。

---

# 13. Streaming Renderer

## 13.1 第一版建议先实现非 streaming

```text
HtmlNode
 -> render fully
 -> byte buffer
 -> HTTP response
```

先确保：

- escape 正确；
- async component 正确；
- error propagation 正确；
- generation lifetime 正确。

## 13.2 第二阶段实现 streaming

目标：

```text
HtmlNode tree
  ↓
Async renderer
  ↓
chunks
  ↓
HTTP response stream
```

## 13.3 Streaming 约束

一旦 headers 已发送：

- 后续 render error 无法切换 status；
- fallback 只能写入 stream 或中断连接。

因此 production renderer 要定义：

```text
prelude buffering threshold
```

例如在发 headers 前至少完成 root 的同步部分。

---

# 14. Fragment 规范

Fragment 是 TSP 的一等能力，但不成为另一种文件类型。

## 14.1 定义

```tsx
import { fragment } from "tsp:server";

export const userList = fragment(async (ctx) => {
  const users = await ctx.services.db.users.list();
  return <UserList users={users} />;
});
```

`fragment()` 返回带 internal symbol/brand 的 callable object。

## 14.2 Fragment endpoint

runtime 在 module evaluate 后记录：

```text
PageSlot.fragments["userList"]
```

## 14.3 Fragment URL 不进入公共协议

用户必须通过：

```ts
ctx.fragment("userList")
```

或未来：

```ts
fragmentUrl(userList)
```

获取 URL。

不要让 application 依赖内部 path，例如：

```text
/__tsp/fragment/...
```

## 14.4 Fragment HTTP method

current contract 推荐默认：

```text
GET + POST
```

也可以显式声明：

```ts
export const save = fragment({
  method: "POST",
  handler: async (ctx) => ...,
});
```

为简化 API，第一版可以先只允许：

```ts
fragment(handler)
```

并继承当前请求 method 或统一 POST；实现前需最终冻结。

**推荐最终方案：fragment 显式 method，默认 GET。**

---

# 15. Layout 设计

current contract 不建议实现复杂 nested layout runtime。

推荐第一版只使用普通 component：

```tsx
import { AppLayout } from "../components/AppLayout.tsx";

export function GET(ctx) {
  return (
    <AppLayout>
      <h1>Users</h1>
    </AppLayout>
  );
}
```

优点：

- 没有隐式 route tree 执行顺序；
- 没有 layout generation ownership 问题；
- 普通模块依赖图足够处理 reload。

未来如需 directory layout，可以在 future releases 设计，不进入 current contract ABI。

---

# 16. `tsp:*` Builtin Modules

TSP runtime API 通过 virtual/builtin modules 暴露。

建议第一版：

```text
tsp:server
tsp:html
tsp:runtime
```

不要大量拆分。

## 16.1 `tsp:server`

导出：

```ts
Context
PageConfig
fragment
json
redirect
text
html
notFound
HttpError
```

## 16.2 `tsp:html`

导出：

```ts
HtmlNode
TrustedHtml
raw
escape
```

## 16.3 `tsp:runtime`

只暴露确实需要给应用查看的 runtime metadata：

```ts
runtime.version
runtime.env
runtime.development
```

不要暴露可破坏 reload/module registry 的内部 API。

## 16.4 不使用 globalThis framework magic

删除概念：

```text
globalThis.Page
globalThis.Fragment
globalThis.__tspBuiltins
```

框架 API 必须显式 import 或通过 Context 获取。

---

# 17. Service Registry / DI

## 17.1 目标

current 走 **PHP-FPM 风格 per-request fresh state**——每个请求得到全新的 page module
scope（`bun:main` 由 `clear_entry_point` + `load_entry_point` 重新 evaluate，见
`docs/reference/bugs/0001`），应用代码不应依赖跨请求共享的 module state。

唯一允许"跨请求持久"的是 **host 持有的基础设施**——它们有自己的生命周期理由
（进程级 logger、session backend 内存/redis 句柄、配置），由 host 进程单例持有，
page 通过 `ctx.services.*` 拿到 **per-request view**（同一个 host 单例的快照，
不持有 host 端 mutable 状态）。

模型：

```text
Native Runtime (process singleton, host-owned)
  └── ServiceRegistry
        ├── logger          # logger 全局单例是合理的（同一 sink 不该被 reset）
        └── sessionStore    # session 句柄单例是合理的（句柄复用以支持持久化）

Per-request page module scope (eval per request, never shared)
  ├── mysql.*             # LIBRARY, page 自己 .createConnection() per request
  ├── password.*         # LIBRARY, page 自己 .hash() per request (bun:password builtin)
  ├── zod.*               # LIBRARY, page 自己 .parse() per request
  └── application code    # page 自己的状态，per request

RequestContext
  └── ServiceView (snapshot of host registry, read-only descriptor for page)
        ↓
JSC ctx.services
```

**关键设计原则**：
- **Libraries vs Services 分清**——zod / mysql2 是 stateless libraries；password 走 bun 内置的 Bun.password (零 embed)，
  page 用 namespace import 直接调用，不要走 service registry
- **Db connection 不该走单例**——PHP-FPM 30 年没单例 db connection，page 每次
  `mysql.createConnection(config)` 拿新连接、用完 close。current 同款
- **Page 侧 module 全部 per-request**——任何"page 自己持有的可变 state"在请求结束
  就该被 GC。强行跨请求持久 = 跟 current 架构冲突

## 17.2 Service 生命周期

只承认两档：

```text
host singleton   process lifetime（host 持有，跨请求）
per-request      page module scope（每次请求重新 evaluate）
```

不要 `transient each resolve`——current page module 本身就在每请求重 evaluate，模块级
const 已经是 per-request 了，再加 transient 档位是冗余。

## 17.3 Libraries (推荐) vs Service adapters (anti-pattern)

对于 mysql2、Redis 等现成 JS 生态，**current 走 namespace library 路径而不是 service
adapter**：

```ts
// page 内：
import { sql } from "tsp:server";
import { zod } from "tsp:server";
import { password } from "tsp:server";  // bun:password builtin (bcrypt / argon2id / scrypt)

const conn = await sql("mysql://" + process.env.DB_PW + "@host/db");
const User = zod.object({ id: zod.number(), name: zod.string() });
const [user] = await conn`SELECT id, name FROM users WHERE id = ${42}`;
const hash = password.hashSync("hunter2", { algorithm: "bcrypt", cost: 10 });
conn.close();
```

实现方式：zod / mysql 走 nanoid 同款 `include_str!` embed（mysql2 / zod 整包源码
+ 它的纯 JS 依赖），prelude 注入 `__tspServer.zod / __tspServer.mysql` namespace；**密码哈希不走 embed**——bun 内置的 `Bun.password`（native Rust）已经覆盖 bcrypt / argon2id / scrypt，page 通过 `import { password } from "tsp:server"` 拿到的 `__tspServer.password === Bun.password`，零 byte 嵌入，零 per-request parse 开销
namespace。**page 自己管 lifecycle**——每请求 new connection、用完 close。

v1 那种 `registerDep("createMySQL", builder)` 的写法**不适用 current**：
- v1 lazy factory 省的是 import 加载成本，current wrap per-request 重新 evaluate，库代码每
  请求都跑，lazy 没意义
- v1 singleton service 通过 `globalThis` 持久，current 明确禁止 framework API 挂
  `globalThis`（§16.4）
- v1 的 "host 持 pool、page 借 connection" 模式是 Java/.NET 思路，**不是 PHP-FPM 思路**

## 17.4 Native service

current 走 native 化的是 host 侧基础设施：

```text
logger          # 日志
session         # session backend（memory / redis 句柄）
runtime         # runtime.version / env / development 元信息
```

**db / redis / zod / etc. 不走 native service**——它们已经在 page 侧走
namespace library 路径（§17.3）。v1 列的"DB drivers / crypto"在 current 重新归类：
纯 JS 实现能 cover 的全部走 library；必须 native 的等真有需要时再说。

## 17.5 application service 定义（host 级，仅限跨请求有意义的）

只对**真正需要跨请求持久**的服务才走这条：

```ts
// tsp.config.ts
export default defineConfig({
  services: {
    mailer: defineService(...),    // 跨请求复用 SMTP 连接
    metricsSink: defineService(...),// metrics 上报
  },
});
```

**不要把 db / redis 放这里**——它们的 lifecycle 是 per-request（§17.1）。v1 把 db
放 services 是个错误设计，current 修正。

如果 host 需要 JS bootstrap（要解释用户写的 service），自定义 JS service
加载到 persistent realm。但 current 首版默认走 TOML/JSON 配置 + host 实现。

---

# 18. Session

Session 建议 native ownership。

## 18.1 API

```ts
interface Session {
  readonly id: string | null;

  get<T = unknown>(key: string): T | undefined;
  set(key: string, value: unknown): void;
  delete(key: string): void;
  clear(): void;
  regenerate(): Promise<void>;
  destroy(): Promise<void>;
}
```

## 18.2 Session serialization

默认只允许 JSON-compatible 数据。

禁止存放：

```text
function
JSC object reference
native pointer
Page module object
```

这样 session 与 page generation 完全解耦。

## 18.3 Store

current contract 建议：

```text
memory
redis
```

接口 native 抽象：

```rust
trait SessionStore {
    async fn load(...);
    async fn save(...);
    async fn delete(...);
}
```

---

# 19. Cookies

提供 native Cookies wrapper：

```ts
ctx.cookies.get("sid")
ctx.cookies.set("name", "value", options)
ctx.cookies.delete("name", options)
```

Set-Cookie 最终由 Rust response builder 统一合并。

防止页面自己构造多个 header 时覆盖 runtime cookies。

---

# 20. Module Runtime

这是 TSP 最关键的底层系统之一。

## 20.1 Canonical Module Identity

每个 application module 必须拥有唯一 canonical identity。

建议：

```text
file:///absolute/canonical/path.tsx
```

Windows 必须规范：

- drive letter；
- slash；
- case strategy；
- symlink policy。

同一物理文件不得因为路径写法不同出现多个 registry entry。

## 20.2 ModuleGraph

Rust 维护：

```rust
struct ModuleGraph {
    nodes: HashMap<ModuleId, ModuleNode>,
    reverse: HashMap<ModuleId, SmallVec<ModuleId>>,
}
```

Node 至少：

```rust
struct ModuleNode {
    id: ModuleId,
    path: PathBuf,
    imports: Vec<ModuleId>,
    page_roots: SmallSet<PageId>,
    source_hash: SourceHash,
}
```

## 20.3 PageRegistry

```rust
struct PageSlot {
    route_id: RouteId,
    source: ModuleId,
    current: Option<GenerationHandle>,
    last_known_good: Option<GenerationHandle>,
    state: PageState,
}
```

## 20.4 PageState

建议：

```rust
enum PageState {
    Unloaded,
    Clean,
    Dirty,
    Building,
    Failed,
}
```

不要用多个 boolean 表示状态。

---

# 21. Generation 模型

## 21.1 核心要求

页面更新必须是 atomic publish。

禁止：

```text
先删旧模块
-> 编译新模块失败
-> 页面直接挂掉
```

正确流程：

```text
current generation N
      ↓ file changed
build candidate N+1
      ↓
resolve
transpile
instantiate
evaluate
validate exports
      ↓ success
atomic publish N+1
      ↓
new requests use N+1
old requests retain N
```

失败：

```text
candidate N+1 failed
      ↓
current remains N
      ↓
dev reports error
production serves LKG N
```

## 21.2 GenerationHandle

```rust
struct Generation {
    id: GenerationId,
    page: PageId,
    module_namespace: JscProtectedValue,
    dependencies: Vec<ModuleId>,
    created_at: Instant,
}
```

实际 JSC value 生命周期必须遵循 Bun/JSC API，不应跨线程直接访问。

## 21.3 Request pinning

请求进入 PageSlot 时：

```text
acquire current GenerationHandle
```

请求完成：

```text
release handle
```

旧 generation 只有在：

```text
not current
AND active_requests == 0
AND no runtime references
```

时才允许释放。

---

# 22. 热更新算法

## 22.1 文件变化

Watcher 产生：

```text
Changed(path)
```

Runtime：

```text
canonicalize path
 -> module id
 -> reverse graph
 -> affected PageRoots
 -> mark PageSlot dirty
```

## 22.2 Lazy reload

默认推荐 **lazy reload**：

文件变化只标 dirty。

下一请求进入时构建 candidate。

优点：

- 避免开发时连续保存触发大量 build；
- 不需要 watcher thread 主动进入 JSC；
- reload 总是在请求执行 owner VM 中完成。

## 22.3 Optional eager reload

后续可支持 development：

```toml
[dev]
reload = "eager"
```

但不是首版核心要求。

## 22.4 In-flight dedup

同一 PageSlot dirty 时并发 20 个请求：

只能有一个 build candidate。

其他请求：

- dev：await 同一个 build future；
- production：可配置继续使用 LKG 或 await。

> **Implementation note (slice 12, 2026-08-24):** the original
> "推荐生产默认：继续 LKG" recommendation is **superseded**.
> Slice 12 chose to await the shared build future for both dev
> and prod -- concurrent waiters all serve the new body once
> the build completes, instead of seeing stale LKG. Rationale:
> LKG is a stale-by-construction body, while a successful build
> is the correct answer; "fall back to LKG" only matters if the
> build fails, which `InFlightBuild::Done(Failed)` handles
> explicitly (host serves LKG only on Failed/Abandoned). The
> spec §32.4 contract ("deduplicate the in-flight build or
> provide equivalent correctness") is satisfied by the await
> path. The original plan recommendation was a default that
> turned out to optimise for a wrong priority (latency over
> freshness); the new default is documented in `docs/reference/progress.md`
> slice 12.

---

# 23. Dependency Graph 更新

每次 candidate 成功后：

```text
old deps
new deps
```

执行 diff：

```text
removed edges -> delete reverse refs
added edges   -> insert reverse refs
```

只有 candidate 成功才 publish graph 变更。

失败 candidate 不能污染 current graph。

这要求 graph transaction 与 generation publish 绑定。

---

# 24. Last Known Good

## 24.1 Development

默认行为建议：

- response 显示 compile/runtime reload error；
- 保留 LKG；
- 可通过 dev UI 选择继续查看 LKG。

## 24.2 Production

生产修改外部 `pages/` 后，如果新版本编译失败：

- 日志记录 error；
- metrics increment；
- 请求继续走 LKG；
- 不向客户端泄露 compiler stack。

如果页面从未成功加载且首次 build 就失败：

```text
500
```

---

# 25. Runtime / JSC 边界

## 25.1 原则

所有 JSC value 的创建、调用、保护、释放应位于 VM owner thread / 合法 runtime context。

禁止从任意 Rust worker 线程直接操作 JSC value。

## 25.2 建议执行模型

首版优先：

```text
one TSP VM per worker
```

每个 worker：

```text
HTTP event loop
JSC VM
PageRegistry (worker-local generation handles)
Persistent JS service realm
```

跨 worker 的 session/DB 等使用真正共享外部资源或各 worker pool。

## 25.3 不建议外置 axum/hyper + 单独 JSC worker

若 TSP 基于 Bun fork，优先复用 Bun 自身 HTTP/runtime/event-loop 能力。

否则需要自行处理：

```text
Rust HTTP threads
  ↕ async channel
JSC VM thread
```

并且会重复实现 Request/Response/stream bridge。

current 更推荐直接在 Bun Rust runtime 内实现 TSP host，或作为 fork 中隔离的 TSP native subsystem。

---

# 26. 推荐 Rust 代码组织

如果继续使用 Bun fork，建议把 TSP 代码隔离，避免散落修改 Bun 核心。

概念目录：

```text
src/runtime/tsp/
├── mod.rs
├── host.rs
├── cli.rs
├── config.rs
├── router.rs
├── route_tree.rs
├── request.rs
├── response.rs
├── cookies.rs
├── session.rs
├── services.rs
├── static_files.rs
├── security.rs
├── jsx.rs
├── page_registry.rs
├── module_graph.rs
├── loader.rs
├── resolver.rs
├── generation.rs
├── invalidation.rs
├── watcher.rs
├── jsc_bridge.rs
├── errors.rs
└── metrics.rs
```

如果 Bun monorepo crate 边界允许，进一步拆为独立 crate：

```text
crates/tsp_host/
crates/tsp_runtime/
```

目标是未来 rebase Bun upstream 时，冲突集中在少数 hook。

---

# 27. TSP Native Entry Point

current 最终不需要：

```text
src/main.ts
```

命令：

```bash
tspserver
```

或 Bun fork 子命令：

```bash
bun tsp
```

我更推荐独立产品名：

```bash
tsp
```

开发：

```bash
tsp dev
```

生产：

```bash
tsp serve
```

构建：

```bash
tsp build
```

检查：

```bash
tsp check
```

路由：

```bash
tsp routes
```

---

# 28. 配置格式

如果目标是完全去掉 JS bootstrap，核心配置最好不是 `tsp.config.ts`。

建议：

```text
tsp.toml
```

示例：

```toml
[server]
host = "0.0.0.0"
port = 3000
workers = 1

[app]
root = "."
routes = "pages"
public = "public"

[dev]
watch = true
reload = "lazy"

[http]
max_body_size = "10mb"
request_timeout_ms = 30000

[session]
driver = "memory"
cookie = "tsp.sid"

[logging]
level = "info"
access = true
```

## 28.1 JS/TS 配置扩展

未来如果必须支持用户代码配置，可增加：

```text
tsp.config.ts
```

但它应运行在 **persistent config realm**，不是 page graph。

首版能不用就不用。

---

# 29. Environment

通过：

```ts
ctx.services.env
```

或：

```ts
import { env } from "tsp:runtime";
```

访问。

可以继续兼容：

```ts
process.env
```

但框架自身不依赖它作为唯一 API。

对 secrets 不做热更新隐式行为，配置变更应由 ConfigManager 明确处理。

---

# 30. Static Files

Rust native 实现。

目录默认：

```text
public/
```

请求：

```text
/assets/app.css
```

优先级建议：

```text
reserved internal routes
static files
application routes
404
```

或者 route 优先于 static，需要在 current 协议中明确。

**推荐：static explicit `/assets` 或 public mount，避免同路径冲突。**

支持：

- ETag；
- Last-Modified；
- If-None-Match；
- Range（可第二阶段）；
- compression；
- immutable cache for hashed asset。

必须防止 directory traversal。

---

# 31. Path Security

所有 filesystem path 在 Rust 层处理。

规则：

1. URL decode 后再 canonical security check；
2. 拒绝 `..` escaping root；
3. 拒绝 NUL；
4. Windows UNC / drive prefix 特别处理；
5. symlink policy 必须配置/固定；
6. 禁止从 pages/public 之外读取 server source，除非显式 API。

不要让 JS 页面自己做 framework-level path normalization。

---

# 32. Error Model

需要稳定的错误码体系。

建议：

```text
TSP1xxx routing/config
TSP2xxx module/loader
TSP3xxx page/runtime/render
TSP4xxx service/session
TSP5xxx build/package
```

例如：

```text
TSP1004 ambiguous route
TSP2003 importing .tsp is forbidden
TSP2011 module transpile failed
TSP2018 generation publish failed
TSP3001 invalid handler result
TSP3102 invalid JSX child
TSP4005 session store unavailable
```

## 32.1 Dev Error Page

开发模式显示：

- error code；
- message；
- source file；
- line/column；
- code frame；
- dependency chain；
- affected PageRoot；
- current generation；
- candidate generation；
- LKG available 状态。

## 32.2 Source Map

TS/TSX runtime stack 必须映射回原始 source。

这是 `.tsp` 保持标准 TSX 的一个关键收益。

---

# 33. Logging

Native logger。

事件至少包括：

```text
server_started
request_completed
route_not_found
page_load_started
page_load_succeeded
page_load_failed
page_generation_published
page_generation_retired
module_invalidated
session_error
service_error
```

Structured JSON 模式：

```json
{
  "level": "info",
  "event": "request_completed",
  "method": "GET",
  "path": "/users",
  "status": 200,
  "duration_ms": 3.82,
  "route": "/users",
  "generation": 42
}
```

---

# 34. Metrics

建议 native 暴露 metrics hooks，至少内部统计：

```text
http_requests_total
http_request_duration
page_reload_total
page_reload_failed_total
page_generation_current
page_generation_retained
module_graph_nodes
module_graph_edges
session_operations
active_requests
```

首版可以只记录，不一定暴露 Prometheus endpoint。

当前实现额外暴露 `/__tsp/metrics`，并由
`scripts/benchmark-tspserver.ps1` / `.sh` 固定记录 cold、p50、p95、p99
基线；指标状态仍由 native host 持有，不进入 page generation。

---

# 35. Worker 模型

current contract 推荐先实现：

```text
workers = 1
```

把 generation correctness 做对。

然后支持多 worker。

多 worker 下有两个选择：

### A. 每 worker 独立 watcher + generation

实现简单，但重复编译。

### B. watcher coordinator + worker invalidate broadcast

推荐最终方案。

```text
Watcher coordinator
   ↓ changed ModuleId
worker 1 mark dirty
worker 2 mark dirty
worker 3 mark dirty
```

当前实现提供可选的 `TSP_INVALIDATION_FILE` append-only bus。每个 worker
只广播 changed path，并在本地重新计算 affected PageSlot；不会跨进程共享
JSC value、module namespace 或 generation。

每个 worker 自己在 VM 内 build generation。

不要尝试跨 JSC VM 共享 module namespace。

---

# 36. Development Runtime

`tsp dev`：

```text
watch = on
LKG = on
source maps = on
dev error page = on
cache = minimal
access log = optional
```

文件变化不重启整个 HTTP server。

目标：

```text
edit .tsx dependency
 -> watcher event
 -> affected PageSlot dirty
 -> next request candidate reload
 -> atomic publish
```

session / DB / logger 全部保持。

---

# 37. Production Runtime

`tsp serve`：

```text
watch = default off
source map reporting = sanitized
LKG = configurable
strict config
structured logging
```

如果用户需要 production live source replacement：

```toml
[production]
watch = true
last_known_good = true
```

但默认部署方式仍建议 immutable application release。

---

# 38. Build / Packaging

## 38.1 目标输出

```text
dist/
├── tspserver        # executable
├── tsp.toml
├── pages/
├── components/
├── lib/
├── public/
└── node_modules/    # 仅实际需要的 external deps，或 runtime package store
```

如果仍采用单 exe + 外部 source：

```text
TSP executable contains runtime/transpiler/JSC.
Application TS/TSX remains external.
```

## 38.2 不 bundle `.tsp` page graph

核心设计仍然是：

```text
external mutable source
 -> runtime transpile
 -> JSC execute
```

否则 hot reload / external source replacement 失去意义。

## 38.3 Dependencies

需要明确 production module resolution strategy：

方案优先级：

1. Bun-compatible package installation layout；
2. build 阶段 vendor application dependencies；
3. 后续实现 package snapshot/store。

current contract 不应同时发明新的 npm package manager。

---

# 39. Type Checking

runtime transpile 不等于 full type checking。

命令：

```bash
tsp check
```

执行：

- TypeScript type check；
- TSP route rules；
- `.tsp` import restriction；
- handler export types；
- fragment metadata；
- JSX runtime typings；
- config validation。

开发服务器可以选择 background diagnostics，但不阻塞每次请求。

---

# 40. TypeScript Types

建议发布一个类型包，由 builtin module resolver 和编辑器共同使用：

```text
@tsp/types
```

或者直接由 runtime distribution 提供 declaration files。

核心：

```text
tsp:server.d.ts
tsp:html.d.ts
tsp:runtime.d.ts
tsp/jsx-runtime.d.ts
tsp/jsx-dev-runtime.d.ts
```

JSX namespace：

```ts
declare namespace JSX {
  type Element = HtmlNode;
  interface IntrinsicElements { ... }
}
```

第一版可以从标准 HTML attributes type 自动生成。

---

# 41. HTML 标准与安全

Renderer 必须默认 escape：

- text node；
- attribute value；
- URL attribute 特殊规则可逐步增加。

禁止隐式 innerHTML。

Raw HTML 必须经过：

```ts
raw(...)
```

并明确标记为 trusted。

对以下属性可增加开发警告：

```text
href="javascript:..."
src="javascript:..."
```

但不在首版实现复杂 HTML sanitizer。

---

# 42. HTTP Method 行为

如果 route 存在但 method export 不存在：

```text
405 Method Not Allowed
Allow: GET, POST
```

如果定义 GET 但没有 HEAD：

建议自动使用 GET 生成 HEAD，丢弃 body。

OPTIONS：

如果没有显式 `OPTIONS`，runtime 可以自动响应：

```text
204
Allow: ...
```

这两个行为应写进协议，避免每页重复实现。

---

# 43. 404 / 500

current contract 建议先使用 runtime 内建页面。

后续可增加：

```text
pages/_404.tsp
pages/_500.tsp
```

但这两个页面的执行错误必须有 native fallback，防止递归错误。

首版可以不实现用户自定义 error page，以减少 lifecycle complexity。

---

# 44. Middleware

我建议 **current contract 不实现通用 JS middleware chain**。

这是很多框架复杂度来源。

先使用明确机制：

```text
native security
page config auth
route handler
service hooks
```

以后如果需要，可以设计：

```text
middleware.ts
```

但必须先定义它与 Page generation 的 ownership。

不要在 current 第一版引入 Express 风格：

```ts
(req, res, next)
```

---

# 45. Authentication

框架只定义 hook，不内置具体认证体系。

例如 native route execution：

```text
resolve route
 -> read PageConfig.auth
 -> auth service hook
 -> handler
```

`ctx.services.auth` 可由用户配置。

这样 auth 不需要通过 middleware magic 实现。

---

# 46. Runtime Initialization

启动顺序必须 deterministic：

```text
1. parse CLI
2. load tsp.toml
3. validate config
4. initialize logger
5. initialize runtime/JSC
6. initialize persistent services
7. scan route tree
8. initialize watcher
9. bind HTTP listener
10. ready
```

关闭：

```text
1. stop accepting new connections
2. drain active requests
3. flush sessions/logs
4. shutdown services
5. release JSC runtime
6. exit
```

---

# 47. Page Load Pipeline

完整流程：

```text
request
  ↓
route match
  ↓
PageSlot
  ↓
acquire_generation()
  ├─ Clean -> current
  │
  └─ Dirty/Unloaded
       ↓
     build_candidate()
       ↓
     read real filesystem source
       ↓
     resolve imports
       ↓
     build dependency graph candidate
       ↓
     transpile TS/TSX
       ↓
     instantiate JSC ESM
       ↓
     evaluate
       ↓
     inspect/validate exports
       ↓
     commit graph transaction
       ↓
     atomic publish generation
  ↓
construct RequestContext
  ↓
lookup method handler
  ↓
call JSC function
  ↓
await Promise if needed
  ↓
HtmlNode | Response
  ↓
render/write
  ↓
release request generation
```

---

# 48. Export Validation

candidate evaluate 后必须验证：

```text
GET is callable if present
POST is callable if present
...
config is valid
fragment exports are valid
```

如果：

```tsx
export const GET = 123;
```

candidate build 失败，不 publish。

错误：

```text
TSP2015: export GET must be a function
```

---

# 49. Module Top-Level Side Effects

`.tsp` 与依赖模块允许 top-level code，因为这是标准 ESM。

但必须明确语义：

```text
top-level executes once per generation instantiation
```

因此不建议在 page graph 中创建需要永久保留的资源：

```ts
const pool = createPool(...); // 不推荐
```

文档应提示用户使用：

```ts
ctx.services.db
```

否则每次 generation 可能产生新资源。

开发模式可增加 warning heuristics，但不是 correctness 机制。

---

# 50. Persistent JS Realm

为了兼容 npm service adapters，推荐区分：

```text
Persistent Realm
Page Generation Modules
```

Persistent Realm 加载：

```text
service adapters
runtime glue
npm singleton services
```

Page graph 不 invalidate persistent realm。

需要谨慎定义 ESM registry ownership，避免同一个 application module误进入 persistent realm。

---

# 51. Rust 与 JS Service Bridge

Native service object 不直接暴露 raw pointer。

使用：

```text
OpaqueHandleId
```

JSC wrapper：

```text
DbHandle { native_id: 42 }
```

方法调用：

```text
JS db.query(...)
 -> native binding
 -> validate handle
 -> async native op
 -> Promise resolve
```

handle registry 负责资源关闭和非法访问检测。

---

# 52. GC 与资源生命周期

不要依赖 JS GC 作为关键系统资源释放的唯一机制。

例如 DB transaction：

推荐：

```ts
await db.transaction(async tx => {
  ...
});
```

Native runtime 明确 begin/commit/rollback。

而不是：

```ts
const tx = db.begin();
// 等 GC rollback
```

GC finalizer 只能作为安全兜底。

---

# 53. Abort / Timeout

每个请求有：

```ts
ctx.signal
```

触发条件：

- client disconnect；
- request timeout；
- server shutdown；
- explicit cancellation。

Native services 应尽量接收 cancellation。

Page handler timeout：

```text
TSP3008 RequestTimeout
```

如果 handler 的 JS promise 永不 resolve，需要 runtime 有中断策略。

JSC hard interruption 能力需要在实现前专项验证。

---

# 54. Concurrency

不能假定一个 page module handler 永远串行。

同一 generation 可能并发处理多个请求。

应用模块顶层 mutable global：

```ts
let counter = 0;
```

其语义是 worker/generation-local shared state。

文档应明确不推荐拿它存 session/user state。

---

# 55. Cache

current contract 不建议先做复杂 framework data cache。

先实现 HTTP cache metadata：

```ts
export const config = {
  cache: "no-store"
};
```

以及用户自己通过 service 使用 Redis/cache。

后续再设计 page output cache，避免和 generation/hot reload 混在一起。

---

# 56. Development Inspector

可以规划但不阻塞首版。

未来 `/__tsp` 可展示：

- routes；
- PageSlot state；
- current generation；
- dependency graph；
- dirty modules；
- reload failures；
- active requests；
- service status。

只在 development 开启。

---

# 57. Security Boundary

TSP application 本质上是服务器代码，默认拥有服务器进程能力。

current 不宣称 sandbox。

如果未来要权限系统，应是另一个独立设计。

不要在首版同时实现 Deno 风格 permission model。

---

# 58. 推荐项目结构

```text
my-app/
├── tsp.toml
├── pages/
│   ├── index.tsp
│   ├── login.tsp
│   └── users/
│       ├── index.tsp
│       └── [id].tsp
├── components/
│   ├── AppLayout.tsx
│   └── UserCard.tsx
├── lib/
│   ├── validation.ts
│   └── format.ts
├── services/
│   └── users.ts
├── public/
│   ├── app.css
│   └── app.js
├── package.json
└── tsconfig.json
```

---

# 59. 一个完整 `.tsp` 推荐示例

```tsx
import {
  type Context,
  type PageConfig,
  fragment,
  redirect,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const users = await ctx.services.users.list();

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="utf-8" />
        <title>Users</title>
        <link rel="stylesheet" href="/assets/app.css" />
      </head>
      <body>
        <main>
          <h1>Users</h1>

          <form method="post">
            <input name="name" required />
            <button type="submit">Create</button>
          </form>

          <button
            hx-get={ctx.fragment("list")}
            hx-target="#user-list"
          >
            Refresh
          </button>

          <section id="user-list">
            <UserList users={users} />
          </section>
        </main>
      </body>
    </html>
  );
}

export async function POST(ctx: Context) {
  const form = await ctx.request.formData();

  const name = String(form.get("name") ?? "").trim();

  if (!name) {
    return new Response("name required", { status: 400 });
  }

  await ctx.services.users.create({ name });

  return redirect("/users");
}

export const list = fragment({
  method: "GET",
  async handler(ctx: Context) {
    const users = await ctx.services.users.list();
    return <UserList users={users} />;
  },
});

function UserList({ users }: { users: User[] }) {
  return (
    <ul>
      {users.map(user => (
        <li data-id={user.id}>{user.name}</li>
      ))}
    </ul>
  );
}
```

---

# 60. `.tsp` 规范建议冻结项

在开始大规模 Rust 实现前，建议先冻结以下 12 项：

1. `.tsp` 是标准 TSX。
2. `.tsp` 不允许被 import。
3. route 文件系统映射规则。
4. named HTTP method exports。
5. `HandlerResult = HtmlNode | Response`。
6. `Context` 最小 API。
7. `fragment()` API。
8. `tsp:*` builtin module naming。
9. JSX child/attribute escaping semantics。
10. async component 是否正式支持。
11. page config 字段。
12. generation/LKG 对请求可见的语义。

这些一旦进入用户代码，后续修改成本最高。

---

# 61. 实施阶段

以下顺序是我最推荐的，不建议直接把现有 `main.ts` 一次性翻译成 Rust。

## Phase 0 — 冻结 Contract

产物：

```text
docs/reference/spec.md
docs/reference/tsp-module.md
docs/reference/jsx-runtime.md
docs/reference/context.md
```

完成条件：

- 上述 12 个冻结项都有明确答案；
- 有 10~20 个 `.tsp` example fixture；
- 不开始考虑 v1 兼容。

---

## Phase 1 — Native Skeleton

实现：

```text
Rust CLI
config loader
HTTP listener
route scanner
route matcher
static response
404/405
shutdown
```

此阶段 `.tsp` 可以先返回 hardcoded response。

验收：

```text
tsp dev
GET /
route params
static files
405 Allow
```

---

## Phase 2 — Minimal JSC Page Execution

实现：

```text
read .tsp
transpile TSX
instantiate module
call GET
await Promise
return text/Response first
```

此阶段可以先不做 JSX tree。

验收：

```tsx
export function GET() {
  return new Response("hello");
}
```

---

## Phase 3 — TSP JSX Runtime

实现：

```text
jsx/jsxs/Fragment builtin
HtmlNode
escaping
attributes
function components
async components
renderer
```

验收覆盖：

- nested components；
- arrays；
- null；
- escaping；
- boolean attrs；
- async component；
- invalid object；
- raw HTML。

---

## Phase 4 — Module Graph

实现：

```text
canonical identity
import scanning/resolution
forward edges
reverse edges
PageRoot mapping
```

先只做 graph，不做 reload。

提供 debug dump：

```bash
tsp graph pages/users/index.tsp
```

---

## Phase 5 — Generation + Atomic Reload

实现：

```text
PageSlot
Generation
candidate build
atomic publish
LKG
request pinning
in-flight dedup
```

这是 current 最关键 milestone。

验收：

1. 页面依赖修改后新请求看到新版本；
2. 老请求完成旧版本；
3. compile error 不破坏 LKG；
4. shared dependency 只 dirty affected pages；
5. candidate graph 失败不污染 current graph。

---

## Phase 6 — Watcher

实现：

```text
filesystem watcher
path canonicalization
changed module -> reverse graph -> page dirty
rename/delete/create
```

重点测试 editor atomic-save 行为。

---

## Phase 7 — Context / Request / Response

实现：

```text
Context bridge
URL
params
query
headers
body
formData
cookies
AbortSignal
Response bridge
```

清除 JS framework handler 层。

---

## Phase 8 — Session + Persistent Services

实现：

```text
ServiceRegistry
memory session
Redis session
logger service
persistent JS adapter realm
```

验收：

```text
reload 页面后 session 不丢
reload 页面后 DB/service 不重建
old generation release 不关闭 service
```

---

## Phase 9 — Fragments

实现：

```text
fragment marker
export discovery
internal routing
ctx.fragment()
HTMX fixtures
```

---

## Phase 10 — Production Packaging

实现：

```text
single executable runtime
external pages/components/public
module resolver production mode
Windows
Linux
```

验收：

```text
目标机器不安装 Bun CLI
直接运行 tspserver
修改外部 .tsp
按配置 reload
```

---

## Phase 11 — Tooling

实现：

```text
tsp check
tsp routes
tsp graph
IDE typings
diagnostics
source maps
```

---

## Phase 12 — Performance / Streaming

最后再做：

```text
streaming HTML
buffer pooling
zero-copy static
multipart streaming
module compile cache
route matcher optimization
metrics
multi-worker invalidation broadcast
```

避免过早优化破坏模型。

---

# 62. 测试矩阵

## 62.1 Parser / Loader

- valid `.tsp`；
- syntax error；
- invalid TSX；
- import `.tsp` rejected；
- circular `.ts/.tsx` deps；
- missing dependency；
- npm dependency；
- tsp builtin；
- source maps。

## 62.2 Router

- static；
- index；
- dynamic；
- catch-all；
- conflict；
- percent encoding；
- Unicode；
- trailing slash policy；
- 404；
- 405；
- HEAD；
- OPTIONS。

## 62.3 JSX

- string escape；
- attribute escape；
- array children；
- nested fragment；
- null/boolean；
- number；
- component；
- async component；
- rejected event callback；
- raw HTML；
- malformed return。

## 62.4 Reload

- root `.tsp` change；
- direct dependency；
- nested dependency；
- shared dependency；
- removed import；
- added import；
- compile failure；
- evaluate failure；
- handler shape invalid；
- concurrent requests；
- active old generation；
- rapid repeated saves；
- file delete/restore。

## 62.5 Runtime State

- session survives reload；
- logger survives reload；
- persistent JS service survives reload；
- request context does not survive completion；
- service handle invalidation；
- server graceful shutdown。

## 62.6 Security

- path traversal；
- encoded traversal；
- symlink escape；
- invalid multipart；
- body limit；
- header injection；
- raw HTML behavior；
- cookie serialization。

---

# 63. Benchmark 基线

在功能正确后建立固定 benchmark：

```text
1. static plaintext
2. static file
3. simple JSX page
4. 1000-row JSX list
5. async page with service call mock
6. fragment
7. cold page compile
8. warm page request
9. dependency reload
10. failed reload + LKG
```

分别记录：

```text
throughput
p50/p95/p99 latency
RSS
JSC heap
cold compile ms
reload ms
generation retained count
```

不要只和 v1 比，要分别测：

```text
native HTTP overhead
JSC handler overhead
JSX render overhead
module reload overhead
```

---

# 64. 代码删除目标

当 current 完成后，旧架构中的以下职责不应继续由 application TypeScript host 实现：

```text
main.ts
router.ts
context.ts
static.ts
cookie runtime
session runtime
response orchestration
runtime/module-graph.ts
runtime/tsp.ts
framework-level dependency registry
worker bootstrap
config hot reload
```

不要求文件名完全一致，但职责必须迁出 page/application JS layer。

---

# 65. 可以继续保留在 JS 的东西

以下不必因为 current 就强行 Rust 化：

```text
application .tsp
application .ts/.tsx
application business logic
npm packages
user validation code
JS service adapters
frontend assets
```

原则不是“没有 JavaScript”，而是：

> **框架生命周期和长期状态不依赖一个 application-level JavaScript bootstrap。**

---

# 66. React 的最终决策

推荐 current 默认：

```text
No React runtime dependency.
```

但可以未来提供 compatibility package：

```text
@tsp/react
```

允许某个 handler 显式返回 React render 结果。

这不进入 TSP core ABI。

核心 `HtmlNode` 必须独立于 React。

---

# 67. current contract 建议功能边界

为了能真正完成 current，建议首发只包含：

```text
Rust host
HTTP
filesystem router
.tsp TSX
native public/ static files
GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS
Context
Response
TSP JSX runtime
components
async components
static files
cookies
session
persistent services
module graph
generation reload
LKG
fragments
CLI/config
Windows/Linux packaging
type declarations
source maps
```

明确推迟：

```text
nested layouts
middleware chains
client hydration
RSC
websocket framework
server actions DSL
ORM
complex page cache
permission sandbox
edge runtime
cluster-wide HMR
```

---

# 68. 关键架构风险

## Risk 1 — Bun/JSC 内部 API 稳定性

如果 TSP 直接依赖 Bun 私有 Rust/JSC internal API，后续 upstream rebase 成本可能很高。

对策：

- TSP native code 独立模块/crate；
- 与 Bun core 只保留少数 adapter/hook；
- 给 module loader / VM / HTTP 建 facade；
- TSP code 不到处访问 Bun internal structs。

## Risk 2 — JSC generation value 生命周期

错误处理可能造成：

- use-after-free；
- module namespace 泄漏；
- old generation 永不释放。

对策：

- 所有 generation handle 明确 retain/release；
- 统一在 VM owner thread 操作；
- 压测持续 reload + 并发请求；
- 提供 retained generation metrics。

## Risk 3 — 自研 JSX runtime 范围失控

容易逐渐复制 React。

对策：

- 只做 server HTML；
- 无 hooks；
- 无 reconciliation；
- 无 client state；
- function + async function component 即止。

## Risk 4 — Persistent JS service 与 page ESM registry 污染

对策：

- 明确 persistent realm；
- application route graph resolver 不允许 persistent module 反向 import page source；
- registry ownership 写入 runtime assertion。

## Risk 5 — 一次性 Rust 重写过大

对策：

严格按 Phase milestone 推进，先形成 vertical slice：

```text
HTTP -> .tsp -> JSC -> Response
```

再逐项替换旧 runtime。

---

# 69. Architecture Decision Records 建议

建议为以下问题单独建立 ADR：

```text
ADR-001 .tsp is TSX, not a custom language
ADR-002 .tsp modules cannot be imported
ADR-003 Rust owns server lifecycle
ADR-004 JSC is application execution VM
ADR-005 TSP uses its own server JSX runtime
ADR-006 Page generations are atomic and request-pinned
ADR-007 Persistent resources are outside page graph
ADR-008 Filesystem routing rules
ADR-009 Handler result is HtmlNode | Response
ADR-010 No generic middleware in current contract
```

这些 ADR 能防止开发过程中架构慢慢滑回 v1。

---

# 70. 第一阶段 PoC 推荐范围

如果现在马上开始写，我建议第一个 PoC **只做 7 件事**：

1. Rust 启动 HTTP server；
2. `/` 映射 `pages/index.tsp`；
3. transpile 标准 TSX；
4. JSC instantiate/evaluate；
5. 找 `GET` export；
6. 调用 `GET(ctx)`；
7. 返回 `Response` 或一个最简单的 TSP JSX `<h1>Hello</h1>`。

不要在第一个 PoC 做：

```text
session
DB
redis
fragments
watcher
multi-worker
full JSX attrs
config hot reload
```

第一个技术判断点是：

> **能否让 Rust host 稳定地拥有 HTTP 生命周期，同时 JSC page module 只是可替换 execution generation。**

如果这个模型成立，后面全部顺着它扩展。

---

# 71. 第二个 PoC：Generation

第二个 PoC 专门验证：

```text
GET request A starts with generation 1
source changes
request B builds/publishes generation 2
request B sees generation 2
request A finishes using generation 1
then generation 1 is retired
```

这个 PoC 比 session、router feature 更重要。

因为它决定整个 current hot reload runtime 是否成立。

---

# 72. 最终推荐的核心 API 面

用户需要学习的 TSP 核心 API 应尽量控制在：

```ts
// tsp:server
Context
PageConfig
fragment
json
redirect
text
html
notFound

// tsp:html
HtmlNode
TrustedHtml
raw
```

页面核心协议只有：

```tsx
export const config = ...;
export function GET(ctx) { ... }
export function POST(ctx) { ... }
export const fragmentName = fragment(...);
```

其他能力尽量通过 Web APIs 与 `ctx.services` 获得。

如果 current 最终需要用户先学几十个 framework primitives，说明 runtime 又变复杂了。

---

# 73. 最终架构摘要

```text
┌──────────────────────────────────────────────┐
│                tspserver (Rust)              │
│                                              │
│  CLI / Config / HTTP / Router / Static       │
│  Cookies / Session / Logger / Services       │
│  Watcher / ModuleGraph / PageRegistry        │
│  Generation / LKG / Request lifecycle        │
└──────────────────────┬───────────────────────┘
                       │ native JSC bridge
┌──────────────────────▼───────────────────────┐
│                JavaScriptCore VM             │
│                                              │
│  .tsp                                        │
│  .ts / .tsx                                  │
│  tsp:* bindings                              │
│  optional npm adapters                       │
└──────────────────────┬───────────────────────┘
                       │
                       ▼
               HtmlNode | Response
                       │
                       ▼
┌──────────────────────────────────────────────┐
│          Native HTML / HTTP Response         │
└──────────────────────────────────────────────┘
```

`.tsp` 的定位：

```text
标准 TSX 语法
+ HTTP method exports
+ route-root lifecycle
+ fragment exports
+ TSP server JSX
```

而不是：

```text
一个新的模板语言
一个 React wrapper
一个 JS server bootstrap
```

---

# 74. 最终验收定义（Definition of Done）

TSP 可以宣布核心架构完成，至少需要满足：

### Runtime

- [x] `tspserver` 不依赖 `main.ts`；
- [x] HTTP lifecycle 为 native；
- [x] `.tsp` 可直接 TS/TSX transpile + JSC execute；
- [x] `.tsp` 不可被 import；
- [x] filesystem routing 正确；
- [x] Context/Response ABI 稳定；
- [x] 自研 TSP JSX runtime 可生产 HTML；
- [x] async component 正确；
- [x] native module graph 正确；
- [x] nested dependency reload 正确；
- [x] shared dependency invalidation 正确；
- [x] generation atomic publish 正确；
- [x] old request/new request generation 隔离正确；
- [x] LKG 正确；
- [x] reload 不重启 HTTP server；
- [x] reload 不重建 session；
- [x] reload 不重建 persistent services（host-owned ServiceRegistry/session）；
- [x] generation 可正确回收，无持续泄漏。

### Protocol

- [x] GET/POST/... exports 冻结；
- [x] HandlerResult 冻结；
- [x] Context current ABI 冻结；
- [x] fragment API 冻结；
- [x] JSX escaping/attributes semantics 冻结；
- [x] route naming rules 冻结；
- [x] tsp:* builtin module names 冻结。

### Tooling

- [x] source-aware diagnostics（`tsp://` source URL + original `.tsp` code frame）；
- [x] `tsp check`；
- [x] `.d.ts` 完整；
- [x] IDE TSX 不报 JSX runtime 错误；
- [x] dev compile error 有原始 `.tsp` code frame。

### Packaging

- [x] Windows executable；
- [x] Linux executable packaging/build path；
- [x] 不要求目标机安装 Bun CLI；
- [x] external pages/components/public 可运行；
- [x] production dependencies 可解析。

---

# 75. 一句话技术方向

TSP 不应该是“把 `main.ts` 翻译成 Rust”。

它应该是：

> **重新定义 `.tsp` 为一个稳定的 TSX Server Page Module 协议，再围绕这个协议构建 Rust-native lifecycle、module generation 和 HTTP runtime。**

最终核心模型：

```text
TSP
=
Rust Web Host
+
Rust TSP Module Runtime
+
JavaScriptCore
+
TypeScript/TSX Server Pages
```

这比追求“全部用 Rust 写”更准确：

- Host 和 framework lifecycle 用 Rust；
- 页面与业务代码继续用 TypeScript；
- JSC 是执行引擎；
- `.tsp` 是二者之间的稳定协议边界。
