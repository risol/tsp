// Test 500 error page - syntax error
export default Page(async function() {
  const x = ;
  return <div>test</div>;
});
