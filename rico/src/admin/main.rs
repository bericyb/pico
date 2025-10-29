use log::info;

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Pico admin starting...");

    let temp = "test string".to_string();
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    map.insert("".to_string(), temp);

    match map.get("") {
        Some(r) => info!("result of empty string! {}", r),
        None => info!("expected temp, got nothing..."),
    }

    Ok(())
}
