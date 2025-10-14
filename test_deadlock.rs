use rust_tide_template::config::CFG;
use async_std::task;

#[async_std::main]
async fn main() {
    println!("Testing CFG access deadlock...");

    // This should work - read lock only
    println!("1. Reading CFG with read lock...");
    let log_level = CFG.read().await.log_level.0;
    println!("   Current log level: {:?}", log_level);

    // This might cause deadlock - trying to get write lock while read lock exists
    println!("2. Attempting to get write lock...");
    let mut write_guard = CFG.write().await;
    println!("   Got write lock successfully!");

    // Modify something
    write_guard.log_level = rust_tide_template::config::LogLevel(tracing::Level::DEBUG);
    println!("   Modified log level to DEBUG");

    // Drop the write guard
    drop(write_guard);
    println!("3. Released write lock");

    println!("Test completed successfully!");
}