use sqlx::sqlite::SqlitePool;
use sqlx::query;
use sqlx::prelude::*;
use tokio::prelude::*;
use tokio::stream::StreamExt;
//use std::time::Duration;
use tokio::time::*;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use num_cpus;

#[tokio::main]
async fn main() {
    if std::path::PathBuf::from("cache.db").exists(){
        println!("Removing old database");
        std::fs::remove_file("cache.db");
    }
    println!("Creating database");
    let pool = SqlitePool::builder().max_size(num_cpus::get() as u32).build("sqlite:cache.db").await.unwrap();
    //Enable WAL mode
    query("PRAGMA journal_size_limit=10000000").execute(&pool).await.unwrap();
    query("PRAGMA soft_heap_limit=1000000000").execute(&pool).await.unwrap();
    
    //Table for the geodata tile cache
    query("CREATE TABLE IF NOT EXISTS data (source TEXT, data BLOB)").execute(&pool).await.unwrap();
    query("CREATE INDEX IF NOT EXISTS data_index on data(source)").execute(&pool).await.unwrap();

    //start process for inserts
    for i in 0 .. 2{
        let insert_pool = pool.clone();
        tokio::spawn(async {timeout(Duration::from_secs(10), async{insert(insert_pool).await}).await});
    }

    //start process for requests
    for i in 0 .. 2{
        let insert_pool = pool.clone();
        tokio::spawn(async {timeout(Duration::from_secs(10), async{request(insert_pool).await}).await});
    }

    delay_for(Duration::from_secs(10)).await;
}

async fn insert(pool: SqlitePool){
    let mut ctr:i32 = 0;
    loop{
        let mut data = vec![0u8; 4096];
        thread_rng().try_fill(&mut *data).unwrap();
        let title = ctr.to_string();
        let result = sqlx::query("INSERT INTO data VALUES (?, ?)")
        .bind(&title)
        .bind(&data)
        .execute(&pool)
        .await;
        match result {
            Ok(_) => eprintln!("Inserting packet {}: OK", &title),
            Err(e) => eprintln!("Inserting packet {}: OErr({:?})", &title, e)
        }
        delay_for(Duration::from_millis(10)).await;
        ctr += 1;
    }
}

async fn request(pool: SqlitePool){
    let mut ctr:i32 = 0;
    loop{
        let title = ctr.to_string();
        let mut cursor = sqlx::query("SELECT data FROM data WHERE source = ?")
        .bind(&title)
        .fetch(&pool);

        match cursor.next().await{
            Some(Ok(row)) => {let data: &[u8] = row.get("data"); eprintln!("Fetching {} OK, got {} bytes", &title, data.len())},
            Some(Err(err)) => eprintln!("Fetching {} failed, got error {:?}", &title, err),
            None => eprintln!("Fetching {} returned None", &title)
        }
        delay_for(Duration::from_millis(10)).await;
        ctr += 1;
    }
}
