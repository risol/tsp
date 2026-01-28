import { registerDep, Page } from "./src/injection-typed.ts";
import type { PageContext } from "./src/context.ts";

// 注册 testFunc
registerDep('testFunc' as never, () => {
  return function testFunc() {
    console.log('testFunc called');
    return 'testFunc called';
  };
});

// 创建 mock context
const context: PageContext = {
  method: "GET",
  url: new URL("http://localhost:9000/"),
  headers: new Headers(),
  query: {},
  body: null,
  cookies: {},
  file: "/test.tsx",
  root: "./www",
};

// 测试 Page
const wrappedFn = Page((ctx: PageContext, deps: AppDeps) => {
  console.log('deps:', deps);
  return 'success';
});

console.log('Testing Page...');
const result = await wrappedFn(context);
console.log('Result:', result);
