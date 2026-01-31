export default Page(async function(ctx, { createMySQL, response }) {
  // 创建 MySQL 连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 更新用户
  const affectedRows = await db.update(
    'users',
    { username: 'updated_user' },
    { id: 1 }
  );

  return response.json({
    success: true,
    message: '用户更新成功',
    affectedRows: affectedRows
  });
});
