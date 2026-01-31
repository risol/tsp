export default Page(async function(ctx, { createMySQL, response, logger }) {
  // 创建 MySQL 连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  try {
    // 开启事务
    await db.beginTransaction();

    // 插入第一条记录
    await db.insert('users', {
      username: 'transaction_user_1_' + Date.now(),
      email: 'trans1_' + Date.now() + '@example.com',
      password_hash: 'hashed_password_trans1'
    });

    // 插入第二条记录
    await db.insert('users', {
      username: 'transaction_user_2_' + Date.now(),
      email: 'trans2_' + Date.now() + '@example.com',
      password_hash: 'hashed_password_trans2'
    });

    // 提交事务
    await db.commit();

    logger.info('事务提交成功');

    return response.json({
      success: true,
      message: '事务成功，两条记录已插入'
    });
  } catch (error) {
    // 回滚事务
    await db.rollback();
    logger.error('事务失败，已回滚', error);

    return response.error('Transaction failed: ' + (error as Error).message, 500);
  }
});
