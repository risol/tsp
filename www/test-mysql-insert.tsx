export default Page(async function(ctx, { createMySQL, response }) {
  // 创建 MySQL 连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 插入新用户
  const insertId = await db.insert('users', {
    username: 'new_user_' + Date.now(),
    email: 'new_user_' + Date.now() + '@example.com',
    password_hash: 'hashed_password_new'
  });

  return response.json({
    success: true,
    message: '用户创建成功',
    insertId: insertId
  });
});
