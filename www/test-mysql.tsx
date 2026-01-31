export default Page(async function(ctx, { createMySQL, response }) {
  // 创建 MySQL 连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 查询所有用户
  const users = await db.query('SELECT * FROM users');

  return response.json({
    success: true,
    count: users.length,
    users: users
  });
});
