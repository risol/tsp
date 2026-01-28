import { registerDepBuilder, withDeps } from "./src/injection.ts";
import type { PageContext } from "./src/context.ts";

// 注册 testFunc
registerDepBuilder('testFunc', () => {
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

// 测试 withDeps
const wrappedFn = withDeps((ctx: PageContext, deps: any) => {
  console.log('deps:', deps);
  return 'success';
});

console.log('Testing withDeps...');
const result = await wrappedFn(context);
console.log('Result:', result);
