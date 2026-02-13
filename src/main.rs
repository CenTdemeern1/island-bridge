use std::time::Duration;

use archipelago_rs::{Connection, ConnectionOptions, ConnectionState, Event, Print, RichText};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

const BATCH_SIZE: usize = 10;

#[derive(Deserialize)]
struct RateLimitResponse {
    retry_after: f64,
}

fn start_queue_consumer(
    mut queue: UnboundedReceiver<Message>,
    webhook_url: String,
) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let reqwest_client: Client = Client::new();
        loop {
            let Some(data) = queue.recv().await else {
                return;
            };
            send_webhook_message(&reqwest_client, &webhook_url, data).await
        }
    })
}

// fn start_queue_consumer(mut queue: UnboundedReceiver<Message>) -> JoinHandle<()> {
//     tokio::task::spawn(async move {
//         let reqwest_client: Client = Client::new();
//         let mut previous_data: Option<Message> = None;
//         loop {
//             let should_batch = queue.len() >= BATCH_SIZE;
//             let mut data = if let Some(pdata) = previous_data {
//                 previous_data = None;
//                 pdata
//             } else {
//                 // This is kinda funky because I can't use or_else with async
//                 let Some(data) = queue.recv().await else {
//                     return;
//                 };
//                 data
//             };
//             if should_batch {
//                 for _ in 0..(BATCH_SIZE - 1) {
//                     if queue.len() == 0 {
//                         send_webhook_message(&reqwest_client, data).await;
//                         previous_data = None;
//                         break;
//                     }
//                     let Some(batch_data) = queue.recv().await else {
//                         return;
//                     };
//                     match (&mut data, batch_data) {
//                         (
//                             Message::Simple { content },
//                             Message::Simple {
//                                 content: batch_content,
//                             },
//                         ) => {
//                             *content += "\n";
//                             *content += &batch_content;
//                         }
//                         (
//                             Message::Player { username, content },
//                             Message::Player {
//                                 username: batch_username,
//                                 content: batch_content,
//                             },
//                         ) if *username == batch_username => {
//                             *content += "\n";
//                             *content += &batch_content;
//                         }
//                         (_, batch_data) => {
//                             send_webhook_message(&reqwest_client, data).await;
//                             previous_data = Some(batch_data);
//                             break;
//                         }
//                     }
//                 }
//             } else {
//                 send_webhook_message(&reqwest_client, data).await;
//                 previous_data = None;
//             }
//         }
//     })
// }

async fn send_webhook_message(reqwest_client: &Client, webhook_url: &str, ref data: Message) {
    loop {
        let response = reqwest_client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(data).unwrap())
            .send()
            .await
            .unwrap();
        if response.status().as_u16() != 429 {
            return;
        }
        println!("I AM BEING RATE LIMITED");
        let RateLimitResponse { retry_after } = response.json().await.unwrap();
        tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
    }
}

struct QueueSender(pub UnboundedSender<Message>);

impl QueueSender {
    fn send_webhook_message(&self, data: Message) {
        self.0.send(data).unwrap();
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum Message {
    Simple { content: String },
    Player { username: String, content: String },
}

fn simple_message(content: &str) -> Message {
    Message::Simple {
        content: content.to_owned(),
    }
}

fn player_message(username: &str, content: &str) -> Message {
    Message::Player {
        username: username.to_owned(),
        content: content.to_owned(),
    }
}

fn format_rich_text(rt: &[RichText]) -> String {
    ["-# ".to_string()]
        .into_iter()
        .chain(rt.iter().map(|v| match v {
            RichText::Player(player) => format!("**{}**", player.name()),
            &RichText::Item {
                ref item,
                ref player,
                progression,
                useful,
                trap,
            } => {
                let mut s = format!("**{}**", item.name());
                if useful {
                    s = format!("*{s}*");
                }
                if progression {
                    s = format!("__{s}__");
                }
                if trap {
                    s = format!("~~{s}~~");
                }
                s
            }
            RichText::Location { location, player } => format!("**{}**", location.name()),
            RichText::PlayerName(s)
            | RichText::EntranceName(s)
            | RichText::Color { text: s, .. } => {
                format!("**{s}**")
            }
            RichText::Text(s) => s.to_owned(),
        }))
        .collect()
}

fn get_env(var: &str) -> Option<String> {
    match std::env::var(var) {
        Ok(v) => Some(v),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            panic!("The contents of the environment variable {var} are not valid Unicode.")
        }
    }
}

#[tokio::main]
async fn main() {
    let webhook_url = get_env("ISLANDBRIDGE_WEBHOOK")
        .expect("Missing webhook URL environment variable (check README.md)");
    let ap_url = get_env("ISLANDBRIDGE_AP_URL")
        .expect("Missing Archipelago URL environment variable (check README.md)");
    let ap_slot = get_env("ISLANDBRIDGE_AP_SLOT")
        .expect("Missing Archipelago slot environment variable (check README.md)");
    let ap_password = get_env("ISLANDBRIDGE_AP_PASSWORD");

    let mut ap_options = ConnectionOptions::new().tags(["TextOnly", "TeamTracker", "IslandBridge"]);
    if let Some(ap_password) = ap_password {
        ap_options = ap_options.password(ap_password);
    }

    let mut connection: Connection = Connection::new(ap_url, "", ap_slot, ap_options);

    let (queue_sender, queue_receiver) = unbounded_channel();
    let queue_sender = QueueSender(queue_sender);
    let queue_consumer = start_queue_consumer(queue_receiver, webhook_url);

    queue_sender.send_webhook_message(simple_message("-# IslandBridge starting..."));

    loop {
        for event in connection.update() {
            match event {
                Event::Connected => {
                    queue_sender.send_webhook_message(simple_message("-# Successfully connected!"));
                }
                Event::Print(print) => match print {
                    Print::Chat {
                        data: _,
                        player,
                        message,
                    } => {
                        let message = if player.game() == "Minecraft Fabric" {
                            message.split_once("> ").unwrap_or((&message, &message)).1
                        } else {
                            &message
                        };
                        let message = if message.starts_with("!") {
                            format!("-# {message}")
                        } else {
                            message.to_string()
                        };
                        queue_sender.send_webhook_message(player_message(&player.name(), &message));
                    }
                    Print::ServerChat { data: _, message } => {
                        queue_sender.send_webhook_message(simple_message(&message));
                    }
                    Print::ItemSend { data, .. }
                    | Print::ItemCheat { data, .. }
                    | Print::Hint { data, .. }
                    | Print::Join { data, .. }
                    | Print::Part { data, .. }
                    | Print::Goal { data, .. }
                    | Print::Release { data, .. }
                    | Print::Collect { data, .. }
                    | Print::Countdown { data, .. } => {
                        queue_sender.send_webhook_message(simple_message(&format_rich_text(&data)));
                    }
                    _ => {}
                },
                Event::Error(error) => println!("Error: {error}"),
                _ => {}
            }
        }
        match connection.state_mut() {
            // ConnectionState::Connecting(_connecting) => {
            //     println!("Connecting...");
            // }
            // ConnectionState::Connected(_client) => {
            //     // println!("Connected!");
            // }
            ConnectionState::Disconnected(error) => {
                queue_sender.send_webhook_message(simple_message(&format!(
                    "IslandBridge disconnected: {error}"
                )));
                break;
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    drop(queue_sender);
    queue_consumer.await.unwrap();
}
