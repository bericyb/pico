use std::collections::HashMap;

fn main() -> std::io::Result<()> {
    println!("Starting scratchpad...");

    let temp = "test string".to_string();
    let mut map: HashMap<String, String> = HashMap::new();

    map.insert("".to_string(), temp);

    match map.get("") {
        Some(r) => println!("result of empty string! {}", r),
        None => println!("expected temp, got nothing..."),
    }

    Ok(())
}
