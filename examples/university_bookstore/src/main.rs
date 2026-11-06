use sqlx::{AnyPool, any::{AnyPoolOptions, install_default_drivers}, Row, Column};
use anyhow::{Result, Context};
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // Install default drivers (sqlite, postgres, etc.)
    install_default_drivers();

    // Parse args: [binary] [connection_string] [sql_dir]
    let args: Vec<String> = env::args().collect();
    
    let db_url = if args.len() > 1 {
        &args[1]
    } else {
        "sqlite:bookstore.db"
    };

    // Determine if the database is Postgres based on the connection string
    let is_postgres = db_url.starts_with("postgres://") || db_url.starts_with("postgresql://");

    // Use generated SQL from SSQL compiler
    let sql_dir_default = if is_postgres { 
        "generated_sql/postgres" 
    } else { 
        "generated_sql/sqlite" 
    };

    let sql_dir = if args.len() > 2 {
        &args[2]
    } else {
        sql_dir_default // Default path based on DB type
    };
    
    // Create results directory
    let results_dir = "results";
    fs::create_dir_all(results_dir)?;
    
    println!("📚 University Book Store - SQL Scenarios (Generic Runner)");
    println!("🔌 Connecting to: {}", db_url);
    println!("📂 Loading SQL from: {}", sql_dir);
    println!("💾 Saving results to: {}\n", results_dir);

    // Connect to the database using AnyPool
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .context("Failed to connect to database")?;

    // Generic SQL Runner
    // Iterate over all files in sql_dir and execute them
    let mut entries: Vec<_> = fs::read_dir(sql_dir)?
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "sql"))
        .collect();

    // Sort by filename to ensure order 01, 02, ...
    entries.sort();

    for path in entries {
        let filename = path.file_name().unwrap().to_string_lossy();
        println!("--- Running: {} ---", filename);
        
        let sql = fs::read_to_string(&path)?;
        
        // Skip empty files or comments-only
        if sql.trim().is_empty() {
            println!("(Empty file)\n");
            continue;
        }

        // Execute
        // We use fetch_all to get rows. 
        // For DML (INSERT/UPDATE/DELETE), fetch_all might return empty or error depending on driver.
        // We wrap in a transaction just in case, or just run it.
        // Since we want to persist changes (e.g. INSERT), we should NOT rollback unless error.
        
        let result = sqlx::query(&sql).fetch_all(&pool).await;
        
        let mut output = String::new();
        
        match result {
            Ok(rows) => {
                if rows.is_empty() {
                    let msg = "(No rows returned or DML executed successfully)";
                    println!("{}\n", msg);
                    output.push_str(msg);
                    output.push('\n');
                } else {
                    // Print columns for first row to show structure
                    if let Some(first_row) = rows.first() {
                        let cols: Vec<_> = first_row.columns().iter().map(|c| c.name()).collect();
                        let col_msg = format!("Columns: {:?}", cols);
                        println!("{}", col_msg);
                        output.push_str(&col_msg);
                        output.push('\n');
                    }
                    
                    for (i, _row) in rows.iter().enumerate() {
                        // Print first 5 rows only to avoid spam
                        if i >= 5 {
                            let msg = format!("... ({} more rows)", rows.len() - 5);
                            println!("{}", msg);
                            output.push_str(&msg);
                            output.push('\n');
                            break;
                        }
                        
                        // Generic print: we can't easily print values without knowing types via AnyRow
                        // But we can print that we got a row.
                        let msg = format!("Row {}: (Data present)", i);
                        println!("{}", msg);
                        output.push_str(&msg);
                        output.push('\n');
                    }
                    println!();
                }
            }
            Err(e) => {
                let msg = format!("Error executing {}: {}", filename, e);
                println!("{}\n", msg);
                output.push_str(&msg);
                output.push('\n');
            }
        }
        
        // Write result to file
        let res_path = Path::new(results_dir).join(format!("{}.txt", filename));
        let mut file = File::create(res_path)?;
        file.write_all(output.as_bytes())?;
    }

    Ok(())
}
