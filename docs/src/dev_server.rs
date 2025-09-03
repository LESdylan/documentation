use axum::{routing::get, Router};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Libft Documentation Server - Go to /dist for docs" }))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/dist", ServeDir::new("dist"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Dev server running on http://localhost:3000");
    println!("📖 Documentation available at http://localhost:3000/dist");
    
    axum::serve(listener, app).await.unwrap();
}
