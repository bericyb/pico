use log::info;
use std::collections::HashMap;

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting scratchpad...");

    let temp = "test string".to_string();
    let mut map: HashMap<String, String> = HashMap::new();

    map.insert("".to_string(), temp);

    match map.get("") {
        Some(r) => info!("result of empty string! {}", r),
        None => info!("expected temp, got nothing..."),
    }

    Ok(())
}
