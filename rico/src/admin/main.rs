fn main() -> std::io::Result<()> {
    println!("Pico admin starting...");

    let temp = "test string".to_string();
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    map.insert("".to_string(), temp);

    match map.get("") {
        Some(r) => println!("result of empty string! {}", r),
        None => println!("expected temp, got nothing..."),
    }

    Ok(())
}
