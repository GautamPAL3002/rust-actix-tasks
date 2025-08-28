# rust-actix-tasks

Task Management REST API built with **Rust**, **Actix-web**, and **SQLx (SQLite)**.
Includes: CRUD, validation, proper HTTP status codes, optional JWT auth, logging, Dockerfile, and Postman collection.

## Quick Start

### 1) Prerequisites
- Rust toolchain (stable)
- SQLite (libsqlite3 is usually preinstalled)
- (Optional) Docker

### 2) Clone & Run
```bash
git clone <your-repo-url>
cd rust-actix-tasks
cargo run
```
The server starts at `http://0.0.0.0:8080` using a local `data.db` SQLite file.

### 3) Environment Variables
| Variable | Default | Description |
|---------|---------|-------------|
| `DATABASE_URL` | `sqlite://data.db` | SQLite connection string |
| `BIND_ADDR` | `0.0.0.0:8080` | Server bind address |
| `JWT_SECRET` | *(unset)* | If set, JWT auth is **enabled** |
| `READ_ONLY_WITHOUT_JWT` | `true` | When JWT is enabled, allow **GET** without token |

Create a `.env` file (optional):
```
DATABASE_URL=sqlite://data.db
BIND_ADDR=0.0.0.0:8080
# Set this to enable JWT:
# JWT_SECRET=supersecret
# READ_ONLY_WITHOUT_JWT=true
```

## API Endpoints

### Auth (optional)
- `POST /api/login` → returns JWT when `JWT_SECRET` is set

### Tasks
- `GET /api/tasks` → list all
- `GET /api/tasks/{id}` → get one
- `POST /api/tasks` → create (title required) *(requires JWT if enabled)*
- `PUT /api/tasks/{id}` → update title/completed *(requires JWT if enabled)*
- `DELETE /api/tasks/{id}` → delete *(requires JWT if enabled)*

## cURL Examples

```bash
# Create
curl -X POST http://localhost:8080/api/tasks   -H 'Content-Type: application/json'   -d '{"title":"Learn Rust"}'

# List
curl http://localhost:8080/api/tasks

# Get one
curl http://localhost:8080/api/tasks/1

# Update
curl -X PUT http://localhost:8080/api/tasks/1   -H 'Content-Type: application/json'   -d '{"completed": true}'

# Delete
curl -X DELETE http://localhost:8080/api/tasks/1
```

### With JWT enabled
```bash
# Get a token
curl -X POST http://localhost:8080/api/login   -H 'Content-Type: application/json'   -d '{"username":"gautam","password":"pass"}'

# Use token
TOKEN=... # paste from login
curl -X POST http://localhost:8080/api/tasks   -H "Authorization: Bearer $TOKEN"   -H 'Content-Type: application/json'   -d '{"title":"Secure Task"}'
```

## Project Structure
```
rust-actix-tasks/
  ├─ src/
  │  └─ main.rs
  ├─ migrations/
  │  └─ 001_init.sql
  ├─ postman/
  │  └─ rust-actix-tasks.postman_collection.json
  ├─ Cargo.toml
  ├─ Dockerfile
  └─ README.md
```

## Docker
```bash
# Build
docker build -t rust-actix-tasks .

# Run (persist DB to a local directory)
docker run -p 8080:8080 -v $(pwd)/data:/app/data rust-actix-tasks
```

## Notes
- Uses SQLx without macros for portability—no compile-time DB required.
- SQLite keeps setup super simple. You can switch `DATABASE_URL` to Postgres/MySQL and adjust SQL if needed.
- Logging via `env_logger` (set `RUST_LOG=info` for more output).

---

**Made for quick submission** — just run, test with Postman (file included), and push to GitHub.
