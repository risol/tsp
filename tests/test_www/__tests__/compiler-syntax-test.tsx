/**
 * Test TSP compiler TypeScript syntax support
 * Verify reported compiler limitations
 */

export default Page(async function(ctx) {
  // Test 1: TypeScript type annotations
  const userId: number = 123;
  const userName: string = "test";

  // Test 2: Type assertion (as)
  const errorAsError = ({} as Error).message;

  // Test 3: Optional chaining (?.)
  const userDetail: any = null;
  const isAdmin = userDetail?.is_admin;

  // Test 4: Type annotations in specific positions (catch clause)
  let caughtError: string | null = null;
  try {
    throw new Error("test");
  } catch (error: any) {
    caughtError = error.message;
  }

  // Test 5: Helper functions after return
  function helper1(): string {
    return "helper";
  }

  // Test 6: Arrow functions
  const helper2 = (): string => "arrow helper";

  // Test 7: Import paths (from www/api/auth/ access www root)
  // This may have issues, needs verification

  return (
    <div>
      <h1>Compiler syntax test</h1>
      <p>userId: {userId}</p>
      <p>userName: {userName}</p>
      <p>errorAsError: {errorAsError}</p>
      <p>isAdmin: {isAdmin ? "true" : "false"}</p>
      <p>caughtError: {caughtError}</p>
      <p>helper1: {helper1()}</p>
      <p>helper2: {helper2()}</p>
    </div>
  );
});
