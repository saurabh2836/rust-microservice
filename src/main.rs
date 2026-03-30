use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use mongodb::{bson::doc, Client as MongoClient};
use redis::Commands; // Synchronous Commands
use sea_query::{Expr, Iden, MysqlQueryBuilder, Order, Query};
use sea_query_binder::SqlxBinder;
use sqlx::mysql::MySqlPool;
use std::sync::Arc;
use std::env;
use sea_query::{Table, ColumnDef};
use aws_sdk_dynamodb::{Client as DynamoClient};
use serde_dynamo::to_item;
use aws_config::BehaviorVersion; // Add this import
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_sns::Client as SnsClient;

// --- 1. Identifiers for SeaQuery ---
#[derive(Iden)]
enum Tasks {
    Table,
    Id,
    Title,
    Completed,
}

// --- 2. App Models & State ---
#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, Debug, Clone)]
struct Task {
    #[serde(default)]
    id: i64,
    title: String,
    completed: bool,
}

struct AppState {
    mysql: MySqlPool,
    mongo: MongoClient,
    redis: redis::Client,
    dynamo: DynamoClient,
    sqs: SqsClient,   // New
    sns: SnsClient,   // New
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Configuration from .env
    let port_str = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let mysql_url = env::var("DATABASE_URL").expect("DATABASE_URL missing");
    let mongo_url = env::var("MONGODB_URI").expect("MONGODB_URI missing");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL missing");

    // Connections
    let mysql_pool = MySqlPool::connect(&mysql_url).await.unwrap();
    let mongo_client = MongoClient::with_uri_str(&mongo_url).await.unwrap();
    let redis_client = redis::Client::open(redis_url).unwrap();
    
    initialize_db(&mysql_pool).await;
    let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let dynamo_client = DynamoClient::new(&config);
// --- DYNAMODB CONNECTION CHECK ---
    match dynamo_client.list_tables().send().await {
        Ok(resp) => {
            let names = resp.table_names();
            println!("✅ AWS DynamoDB Connected!");
            println!("📊 Tables found: {:?}", names);
            
            // Optional: Check if your specific "Tasks" table exists
            if !names.contains(&"Tasks".to_string()) {
                println!("⚠️  WARNING: Table 'Tasks' not found in this region. Please create it.");
            }
        },
        Err(e) => {
            // This will catch the AccessDenied or InvalidToken errors immediately
            eprintln!("\n❌ DYNAMODB CONNECTION ERROR:");
            eprintln!("Check your .env credentials and IAM policies.");
            eprintln!("Error: {:?}\n", e);
            std::process::exit(1); // Stop the server if AWS is broken
        }
    }
    // ---------------------------------
    let state = Arc::new(AppState {
        mysql: mysql_pool,
        mongo: mongo_client,
        redis: redis_client,
        dynamo:dynamo_client,
        sqs: SqsClient::new(&config), // Initialize SQS
        sns: SnsClient::new(&config), // Initialize SNS
    });

    // Router Definition
let app = Router::new()
    .route("/tasks", get(list_tasks).post(create_task))
    .route("/tasks/{id}", put(update_task).delete(delete_task)) // Changed :id to {id}
    .with_state(state);

    let addr = format!("127.0.0.1:{}", port_str);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("\n🚀 Multi-DB CRUD with SeaQuery Started");
    println!("📡 Port: {}", port_str);

    axum::serve(listener, app).await.unwrap();
}

async fn initialize_db(pool: &MySqlPool) {
    // Build the "CREATE TABLE IF NOT EXISTS" query using SeaQuery
    let sql = Table::create()
        .table(Tasks::Table)
        .if_not_exists()
        .col(ColumnDef::new(Tasks::Id).big_integer().not_null().auto_increment().primary_key())
        .col(ColumnDef::new(Tasks::Title).string().not_null())
        .col(ColumnDef::new(Tasks::Completed).boolean().not_null().default(false))
        .build(MysqlQueryBuilder);

    // Execute the query
    sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("Failed to create 'tasks' table");
        
    println!("✅ Database table 'tasks' is ready.");
}
// --- 3. Handlers ---

async fn create_task(State(state): State<Arc<AppState>>, Json(mut task): Json<Task>) -> Json<Task> {
    // MySQL: SeaQuery Builder
    let (sql, values) = Query::insert()
        .into_table(Tasks::Table)
        .columns([Tasks::Title, Tasks::Completed])
        .values_panic([task.title.clone().into(), task.completed.into()])
        .build_sqlx(MysqlQueryBuilder);

    let res = sqlx::query_with(&sql, values).execute(&state.mysql).await.unwrap();
    task.id = res.last_insert_id() as i64;

    // B. DynamoDB Insert (JSON Format)
    // We convert our Task struct into a DynamoDB Item
    let item = to_item(task.clone()).expect("Failed to serialize task to DynamoDB item");
    
    state.dynamo
        .put_item()
        .table_name("Tasks")
        .set_item(Some(item))
        .send()
        .await
        .expect("DynamoDB Insert Failed");

    // --- NEW: AWS SQS (Add to background worker queue) ---
    let sqs_url = env::var("SQS_QUEUE_URL").unwrap();
    let message_body = serde_json::to_string(&task).unwrap();
    let _ = state.sqs.send_message()
                .queue_url(&sqs_url)
                .message_body(&message_body)
                .send()
                .await;
        // .expect("SQS Send Failed");
    match state.sqs.send_message()
    .queue_url(&sqs_url)
    .message_body(&message_body)
    .send()
    .await {
        Ok(_) => println!("🚀 Message sent to SQS"),
        Err(e) => eprintln!("⚠️ SQS Warning: Could not send message. Error: {:?}", e),
        // The server keeps running even if SQS fails!
    }


    // --- NEW: AWS SNS (Broadcast notification) ---
    let topic_arn = env::var("SNS_TOPIC_ARN").unwrap();
    state.sns.publish()
        .topic_arn(topic_arn)
        .message(format!("New Task Created: {}", task.title))
        .send()
        .await
        .expect("SNS Publish Failed");

    // Mongo & Redis Audit
    log_sync_redis(&state, "CREATE").await;
    let logs = state.mongo.database("app_db").collection::<mongodb::bson::Document>("audit");
    let _ = logs.insert_one(doc! { "op": "CREATE", "id": task.id }).await;

    Json(task)
}

async fn list_tasks(State(state): State<Arc<AppState>>) -> Json<Vec<Task>> {
    // MySQL: SeaQuery Builder
    let (sql, values) = Query::select()
        .columns([Tasks::Id, Tasks::Title, Tasks::Completed])
        .from(Tasks::Table)
        .order_by(Tasks::Id, Order::Asc)
        .build_sqlx(MysqlQueryBuilder);

    let tasks = sqlx::query_as_with::<_, Task, _>(&sql, values)
        .fetch_all(&state.mysql).await.unwrap();

    log_sync_redis(&state, "LIST").await;
    Json(tasks)
}

async fn update_task(State(state): State<Arc<AppState>>, Path(id): Path<i64>, Json(task): Json<Task>) -> String {
    // MySQL: SeaQuery Builder
    let (sql, values) = Query::update()
        .table(Tasks::Table)
        .values([(Tasks::Title, task.title.into()), (Tasks::Completed, task.completed.into())])
        .and_where(Expr::col(Tasks::Id).eq(id))
        .build_sqlx(MysqlQueryBuilder);

    sqlx::query_with(&sql, values).execute(&state.mysql).await.unwrap();

    log_sync_redis(&state, "UPDATE").await;
    format!("Updated task {}", id)
}

async fn delete_task(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> String {
    // MySQL: SeaQuery Builder
    let (sql, values) = Query::delete()
        .from_table(Tasks::Table)
        .and_where(Expr::col(Tasks::Id).eq(id))
        .build_sqlx(MysqlQueryBuilder);

    sqlx::query_with(&sql, values).execute(&state.mysql).await.unwrap();

    log_sync_redis(&state, "DELETE").await;
    format!("Deleted task {}", id)
}

// --- 4. Sync Redis Helper ---
async fn log_sync_redis(state: &AppState, op: &'static str) {
    let redis_client = state.redis.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = redis_client.get_connection().expect("Sync Redis Failed");
        let _: () = conn.set("last_op", op).unwrap();
    }).await.unwrap();
}