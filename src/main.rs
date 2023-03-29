use serde;
use std::io::Read;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tdlib::{
    enums::{AuthorizationState, Update, User},
    functions,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

mod render;
mod telegram;

use render::*;
use telegram::*;

#[derive(serde::Deserialize)]
pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
}

#[tokio::main]
async fn main() {
    let config: Config = {
        use ron;
        let mut s = String::new();
        let _ = std::fs::File::open("config.ron")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        ron::from_str(&s).unwrap()
    };

    // Create the client object
    let client_id = tdlib::create_client();

    // Create a mpsc channel for handling AuthorizationState updates separately
    // from the task
    let (auth_tx, auth_rx) = mpsc::channel(5);

    // Create a flag to make it possible to stop receiving updates
    let run_flag = Arc::new(AtomicBool::new(true));
    let run_flag_auth_clone = run_flag.clone();
    let run_flag_render_clone = run_flag.clone();

    let mut terminal: tui::Terminal<_> = render::setup().unwrap();
    let (render_tx, render_handle) = render::start_render_thread(terminal, run_flag_render_clone);

    let h_render_tx = render_tx.clone();
    // Spawn a task to receive updates/responses
    let handle = tokio::spawn(async move {
        while run_flag_auth_clone.load(Ordering::Acquire) {
            if let Some((update, _client_id)) = tdlib::receive() {
                handle_update(update, &auth_tx, &h_render_tx).await;
            }
        }
    });

    // Set a fairly low verbosity level. We mainly do this because tdlib
    // requires to perform a random request with the client to start receiving
    // updates for it.
    functions::set_log_verbosity_level(0, client_id)
        .await
        .unwrap();

    // Handle the authorization state to authenticate the client
    let auth_rx = handle_authorization_state(client_id, auth_rx, run_flag.clone(), &config).await;

    // Run the get_me() method to get user information
    let User::User(me) = functions::get_me(client_id).await.unwrap();
    println!("Hi, I'm {}", me.first_name);

    // Tell the client to close
    functions::close(client_id).await.unwrap();

    // Handle the authorization state to wait for the "Closed" state
    handle_authorization_state(client_id, auth_rx, run_flag.clone(), &config).await;
    &render_tx.send(render::RenderUpdate::Exit).await.unwrap();
    // Wait for the previously spawned task to end the execution
    handle.await.unwrap();
    render_handle.await.unwrap();
}
