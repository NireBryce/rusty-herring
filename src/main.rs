use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    // Define routes
    let app = Router::new()
        .route("/", get(home));

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}

// Handler function
async fn home() -> &'static str {
    "It works."
}
