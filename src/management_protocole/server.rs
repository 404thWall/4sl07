use crate::management_protocole::server::OutMsg::MsgClose;
use crate::management_protocole::{CommandCodec, Packet, ProtocolError, Task};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;
use tokio_util::codec::Framed;

static MAP_TASKS_AMOUNT: usize = 20;
static REDUCE_TASKS_AMOUNT: usize = 4;

static LAST_RECEIVED_PING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static TASK_QUEUE: LazyLock<RwLock<Vec<Task>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static MAP_TASKS_FINISHED: LazyLock<RwLock<(Vec<bool>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![false; MAP_TASKS_AMOUNT], 0)));

enum OutMsg {
    MsgPacket(Packet),
    MsgClose,
}

pub async fn start_server(addr: &str) -> Result<(), ProtocolError> {
    let listener = TcpListener::bind(addr).await?;
    generate_map_tasks().await;

    loop {
        let (socket, addr) = listener.accept().await?;

        // Wrap the socket with our codec
        let framed = Framed::new(socket, CommandCodec);

        let (mut sender, mut receiver) = framed.split();
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let writer_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    OutMsg::MsgPacket(packet) => {
                        if let Err(e) = sender.send(packet).await {
                            eprintln!("send error: {}", e);
                            break;
                        }
                    }
                    OutMsg::MsgClose => {
                        println!("Closing connection to {}", addr);
                        sender.close().await.ok();
                        break;
                    }
                }
            }
        });

        let mut ping_tx = tx.clone();
        let ping_task = tokio::spawn(async move {
            server_ping_task(&mut ping_tx, &addr).await;
        });

        let send_back_tx = tx.clone();
        tokio::spawn(async move {
            println!("New connection from {}", addr);

            while let Some(result) = receiver.next().await {
                let response = match result {
                    Ok(cmd) => server_handle_packet(cmd, &addr).await,
                    Err(e) => {
                        eprintln!("Protocol error: {}", e);
                        Err(e)
                    }
                };

                if let Ok(Some(packet)) = response {
                    if let Err(e) = send_back_tx.send(OutMsg::MsgPacket(packet)).await {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }
                }
            }

            send_back_tx.send(MsgClose).await.ok();
            drop(tx);
            ping_task.abort();
            let _ = writer_task.await;

            println!("Connection from {} closed", addr);
        });
    }
}

async fn server_ping_task(tx: &mut Sender<OutMsg>, addr: &std::net::SocketAddr) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        ticker.tick().await;
        if tx.send(OutMsg::MsgPacket(Packet::Ping)).await.is_err() {
            break;
        }
        let value;
        {
            let mut map = LAST_RECEIVED_PING.write().await;
            let key = addr.to_string();
            value = map.get(&key).cloned().unwrap_or(0);
            map.insert(key.clone(), value + 1);
        }
        if value == 3 {
            println!(
                "No Pong received from {} after 3 Pings, closing connection",
                addr
            );
            tx.send(OutMsg::MsgClose).await.ok();
            break;
        }
    }
}

pub async fn server_handle_packet(
    packet: Packet,
    addr: &std::net::SocketAddr,
) -> Result<Option<Packet>, ProtocolError> {
    match packet {
        Packet::Ping => {
            println!("Received Ping from {}, sending Pong...", addr);
            Ok(Some(Packet::Pong))
        }
        Packet::Pong => {
            println!("Received Pong from {}", addr);
            LAST_RECEIVED_PING.write().await.insert(addr.to_string(), 0);
            Ok(None)
        }
        Packet::Connect(server_port) => {
            println!(
                "Received Connect from {} with server port {}",
                addr, server_port
            );
            // Handle Connect if needed
            Ok(None)
        }
        Packet::AskForTask => {
            println!("Received AskForTask from {}", addr);
            let mut queue = TASK_QUEUE.write().await;
            if queue.is_empty() {
                println!("No more tasks available for {}, sending None", addr);
                return Ok(Some(Packet::GiveTask(Task::None)));
            }
            let task = queue.swap_remove(0);
            println!("Assigning task {:?} to {}", task, addr);
            Ok(Some(Packet::GiveTask(task)))
        }
        Packet::GiveTask(task) => {
            println!("Received GiveTask from {} with task: {:?}", addr, task);
            // Handle GiveTask if needed
            Ok(None)
        }
        Packet::TaskFinished(task) => {
            println!("Received TaskFinished from {} for task: {:?}", addr, task);
            match task {
                Task::Map(key, _) => {
                    let mut tuple = MAP_TASKS_FINISHED.write().await; // (vec, count)
                    if tuple.0[key as usize] {
                        println!("Task Map {} was already marked as finished, ignoring", key);
                    } else {
                        println!("Marking Task Map {} as finished", key);
                        tuple.0[key as usize] = true;
                        tuple.1 += 1;
                    }

                    if tuple.1 == MAP_TASKS_AMOUNT as u32 {
                        println!("All Map tasks finished, generating Reduce tasks...");
                        generate_reduce_tasks().await;
                    }
                }
                _ => {}
            }
            Ok(None)
        }
    }
}

async fn generate_map_tasks() {
    let mut tasks = TASK_QUEUE.write().await;

    for i in 0..MAP_TASKS_AMOUNT {
        tasks.push(Task::Map(i as u32, MAP_TASKS_AMOUNT as u32));
    }
}

async fn generate_reduce_tasks() {
    let mut tasks = TASK_QUEUE.write().await;

    for i in 0..REDUCE_TASKS_AMOUNT {
        tasks.push(Task::Reduce(i as u32, REDUCE_TASKS_AMOUNT as u32));
    }

    println!("Tasks in queue: {}", tasks.len());
}
