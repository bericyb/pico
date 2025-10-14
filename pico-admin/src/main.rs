use std::sync::Arc;

use axum::{Router, routing::get};
use handlebars::{DirectorySourceOptions, Handlebars};

struct AppState {
    hbs: Handlebars<'static>,
}

#[tokio::main]
async fn main() {
    let mut hbs = Handlebars::new();
    let mut opts = DirectorySourceOptions::default();
    opts.tpl_extension = ".html".to_string();
    if let Err(e) = hbs.register_templates_directory("templates/", opts) {
        println!("failed to load template directory: {}", e);
        return;
    }

    let app_state = Arc::new(AppState { hbs });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
