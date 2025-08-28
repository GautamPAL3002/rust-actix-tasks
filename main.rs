\
use actix_web::{get, post, put, delete, web, App, HttpResponse, HttpServer, Responder, HttpRequest, middleware::Logger};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use thiserror::Error;
use validator::Validate;
use std::fs;
use std::env;
use chrono::{Utc, DateTime};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};

// ---------- Models ----------

#[derive(Serialize)]
struct Task {
    id: i64,
    title: String,
    completed: bool,
    created_at: String,
}

#[derive(Deserialize, Validate)]
struct CreateTask {
    #[validate(length(min = 1, message = "title cannot be empty"))]
    title: String,
}

#[derive(Deserialize, Validate)]
struct UpdateTask {
    #[validate(length(min = 1, message = "title cannot be empty"))]
    title: Option<String>,
    completed: Option<bool>,
}

// ---------- JWT ----------

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn ensure_auth(req: &HttpRequest, data: &AppState) -> Result<(), AppError> {
    if !data.jwt_enabled {
        return Ok(());
    }
    // Allow GET endpoints without auth if read-only is true
    if data.read_only_without_jwt && req.method() == "GET" {
        return Ok(());
    }
    let auth = req.headers().get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    let token = auth.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    let key = DecodingKey::from_secret(data.jwt_secret.as_ref().expect("jwt enabled").as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<Claims>(token, &key, &validation).map_err(|_| AppError::Unauthorized)?;
    Ok(())
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[post("/api/login")]
async fn login(body: web::Json<LoginBody>, data: web::Data<AppState>) -> Result<impl Responder, AppError> {
    if !data.jwt_enabled {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "JWT not enabled on server (set JWT_SECRET to enable)"
        })));
    }
    // Dummy user check - accept any non-empty username/password
    if body.username.trim().is_empty() || body.password.trim().is_empty() {
        return Err(AppError::Unauthorized);
    }
    let exp = (Utc::now() + chrono::Duration::hours(12)).timestamp() as usize;
    let claims = Claims { sub: body.username.clone(), exp };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(data.jwt_secret.as_ref().unwrap().as_bytes()))
        .map_err(|_| AppError::Internal("Failed to sign token".into()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "token": token,
        "expires_in_hours": 12
    })))
}

// ---------- Errors ----------

#[derive(Error, Debug)]
enum AppError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Not Found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal Server Error: {0}")]
    Internal(String),
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::BadRequest(msg) => HttpResponse::BadRequest().json(serde_json::json!({ "error": msg })),
            AppError::NotFound => HttpResponse::NotFound().json(serde_json::json!({ "error": "Not Found" })),
            AppError::Unauthorized => HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Unauthorized" })),
            AppError::Internal(msg) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": msg })),
        }
    }
}

// ---------- State ----------

struct AppState {
    pool: SqlitePool,
    jwt_enabled: bool,
    jwt_secret: Option<String>,
    read_only_without_jwt: bool,
}

// ---------- Handlers ----------

#[post("/api/tasks")]
async fn create_task(
    req: HttpRequest,
    data: web::Data<AppState>,
    payload: web::Json<CreateTask>,
) -> Result<impl Responder, AppError> {
    ensure_auth(&req, &data).await?;
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let rec = sqlx::query(
        "INSERT INTO tasks (title, completed) VALUES (?, ?) RETURNING id, title, completed, created_at"
    )
    .bind(&payload.title)
    .bind(false)
    .fetch_one(&data.pool).await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let task = Task {
        id: rec.get::<i64, _>("id"),
        title: rec.get::<String, _>("title"),
        completed: rec.get::<i64, _>("completed") != 0,
        created_at: rec.get::<String, _>("created_at"),
    };
    Ok(HttpResponse::Created().json(task))
}

#[get("/api/tasks")]
async fn list_tasks(data: web::Data<AppState>) -> Result<impl Responder, AppError> {
    let rows = sqlx::query("SELECT id, title, completed, created_at FROM tasks ORDER BY id DESC")
        .fetch_all(&data.pool).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let tasks: Vec<Task> = rows.into_iter().map(|rec| Task {
        id: rec.get::<i64, _>("id"),
        title: rec.get::<String, _>("title"),
        completed: rec.get::<i64, _>("completed") != 0,
        created_at: rec.get::<String, _>("created_at"),
    }).collect();
    Ok(HttpResponse::Ok().json(tasks))
}

#[get("/api/tasks/{id}")]
async fn get_task(path: web::Path<i64>, data: web::Data<AppState>) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let rec = sqlx::query("SELECT id, title, completed, created_at FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(&data.pool).await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(rec) = rec {
        let task = Task {
            id: rec.get::<i64, _>("id"),
            title: rec.get::<String, _>("title"),
            completed: rec.get::<i64, _>("completed") != 0,
            created_at: rec.get::<String, _>("created_at"),
        };
        Ok(HttpResponse::Ok().json(task))
    } else {
        Err(AppError::NotFound)
    }
}

#[put("/api/tasks/{id}")]
async fn update_task(
    req: HttpRequest,
    path: web::Path<i64>,
    data: web::Data<AppState>,
    payload: web::Json<UpdateTask>,
) -> Result<impl Responder, AppError> {
    ensure_auth(&req, &data).await?;
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let id = path.into_inner();
    // Fetch existing
    let existing = sqlx::query("SELECT id, title, completed, created_at FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(&data.pool).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if existing.is_none() {
        return Err(AppError::NotFound);
    }
    let current = existing.unwrap();
    let new_title: String = payload.title.clone().unwrap_or_else(|| current.get::<String, _>("title"));
    let new_completed: bool = payload.completed.unwrap_or_else(|| current.get::<i64, _>("completed") != 0);

    let rec = sqlx::query(
        "UPDATE tasks SET title = ?, completed = ? WHERE id = ? RETURNING id, title, completed, created_at"
    )
    .bind(new_title)
    .bind(new_completed)
    .bind(id)
    .fetch_one(&data.pool).await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let task = Task {
        id: rec.get::<i64, _>("id"),
        title: rec.get::<String, _>("title"),
        completed: rec.get::<i64, _>("completed") != 0,
        created_at: rec.get::<String, _>("created_at"),
    };
    Ok(HttpResponse::Ok().json(task))
}

#[delete("/api/tasks/{id}")]
async fn delete_task(req: HttpRequest, path: web::Path<i64>, data: web::Data<AppState>) -> Result<impl Responder, AppError> {
    ensure_auth(&req, &data).await?;
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM tasks WHERE id = ?").bind(id).execute(&data.pool).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}

// ---------- Migrations ----------

async fn run_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = fs::read_to_string("migrations/001_init.sql")
        .map_err(|e| AppError::Internal(format!("Failed reading migration: {}", e)))?;
    sqlx::query(&sql).execute(pool).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

// ---------- Main ----------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".into());
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let jwt_secret = env::var("JWT_SECRET").ok();
    let read_only_without_jwt = env::var("READ_ONLY_WITHOUT_JWT").ok().map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(true);
    let jwt_enabled = jwt_secret.is_some();

    let pool = SqlitePool::connect(&database_url).await
        .expect("Failed to connect to SQLite");

    run_migrations(&pool).await.expect("Migration failed");

    let state = web::Data::new(AppState {
        pool,
        jwt_enabled,
        jwt_secret,
        read_only_without_jwt,
    });

    println!("Server running at http://{}/", &bind_addr);
    println!("JWT enabled: {}", jwt_enabled);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .service(login)
            .service(create_task)
            .service(list_tasks)
            .service(get_task)
            .service(update_task)
            .service(delete_task)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
