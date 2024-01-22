use crate::blockchain::Transaction;
use crate::node::Storage;
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::Mutex;
use tokio::task::{self, JoinHandle};
use tower_http::trace::TraceLayer;

mod api_payload {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    pub struct Transaction {
        pub author: String,
        pub recipient: String,
        pub amount: f64,
        pub signature: String,
    }
}

pub struct ApiServer {
    thread: JoinHandle<()>,
}

impl ApiServer {
    async fn add_transaction_to_mining_pool(
        State(storage): State<Arc<Mutex<Storage>>>,
        Json(transaction): Json<api_payload::Transaction>,
    ) -> impl IntoResponse {
        let mut storage = storage.lock().await;

        storage.minable_transactions.push(Transaction {
            author: transaction.author.into(),
            recipient: transaction.recipient.into(),
            amount: transaction.amount,
            signature: transaction.signature.into(),
        });

        StatusCode::CREATED
    }

    pub async fn new(
        storage: Arc<Mutex<Storage>>,
        address: impl ToSocketAddrs,
    ) -> Result<ApiServer> {
        let listener = TcpListener::bind(address).await?;
        let app = Router::new()
            .route("/v1/mine", post(Self::add_transaction_to_mining_pool))
            .layer(TraceLayer::new_for_http())
            .with_state(storage);

        let thread = task::spawn(async move {
            _ = axum::serve(listener, app).await;
        });

        Ok(ApiServer { thread })
    }
}
