use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use websocket::sync::{Server, receiver, sender};
use websocket::message::OwnedMessage;

fn handle_client(sender: sender, receiver: receiver, clients: Arc<Mutex<HashSet<sender>>>) {
    // Add the client to the set
    clients.lock().unwrap().insert(sender.clone());

    for message in receiver.incoming_messages() {
        match message {
            Ok(OwnedMessage::Close(_)) => {
                // Remove the client from the set and close the connection
                clients.lock().unwrap().remove(&sender);
                sender.send_message(&OwnedMessage::Close(None)).unwrap();
                break;
            }
            Ok(_) => (),
            Err(_) => {
                // Handle errors here
            }
        }
    }
}

fn main() {
    // Create a set to store connected clients
    let clients = Arc::new(Mutex::new(HashSet::new()));

    // Start a WebSocket server
    let server = Server::bind("127.0.0.1:8080").unwrap();

    for request in server.filter_map(Result::ok) {
        if !request.protocols().contains(&"rust-websocket".to_string()) {
            request.reject().unwrap();
            continue;
        }

        // Accept the WebSocket connection
        let response = request.use_protocol("rust-websocket").accept().unwrap();
        let (sender, receiver) = response.send().split().unwrap();
        let clients_clone = clients.clone();

        // Spawn a new thread to handle the WebSocket connection
        thread::spawn(move || {
            handle_client(sender, receiver, clients_clone);
        });
    }

    // Simulate data changes and send updates to clients
    let data = Arc::new(Mutex::new("Initial Data".to_string()));
    loop {
        // Simulate a data change
        let mut data = data.lock().unwrap();
        *data = "Updated Data".to_string();

        // Send updates to connected clients
        let clients = clients.lock().unwrap();
        for client in clients.iter() {
            client.send_message(&OwnedMessage::Text(data.clone())).unwrap();
        }

        thread::sleep(std::time::Duration::from_secs(5));
    }
}
