pub type UserId = i32;

#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub hashed_password: String,
}

/*
pub async fn auth_handler(env: &Env, login: Login)  {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, hashed_password FROM users WHERE username = $1",
    )
    .bind(login.username)
    .fetch_one(&env.pool)
    .await;
}
*/

