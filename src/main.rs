mod client;
mod config;
mod listener;
mod pb;
mod processor;
mod queue;
mod types;

use clap::Parser;
use config::ClientConfig;
use log::{error, info, warn};
use std::env;
use tokio::sync::mpsc;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'f', long)]
    file: Option<String>,

    #[arg(short = 's', long)]
    server: Option<String>,

    #[arg(short = 'p', long)]
    port: Option<u16>,

    #[arg(long)]
    insecure: Option<bool>,

    #[arg(short = 'i', long)]
    interval: Option<u64>,

    #[arg(long)]
    sensor_id: Option<String>,

    #[arg(short = 't', long)]
    testing_mode: Option<bool>,

    #[arg(short = 'k', long)]
    max_clients: Option<usize>,

    #[arg(short = 'm', long)]
    max_message_size: Option<usize>,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load configuration
    let mut conf = ClientConfig::new()?;

    // Override config with command line arguments
    if let Some(file) = args.file {
        conf.file = file;
    }
    if let Some(server) = args.server {
        conf.server = server;
    }
    if let Some(port) = args.port {
        conf.port = port;
    }
    if let Some(insecure) = args.insecure {
        conf.insecure = insecure;
    }
    if let Some(interval) = args.interval {
        conf.interval = interval;
    }
    if let Some(sensor_id) = args.sensor_id {
        conf.sensor_id = sensor_id;
    }
    if let Some(testing_mode) = args.testing_mode {
        conf.testing_mode = testing_mode;
    }
    if let Some(max_clients) = args.max_clients {
        conf.max_clients = Some(max_clients);
    }
    if let Some(max_message_size) = args.max_message_size {
        conf.max_message_size = max_message_size;
    }
    if args.verbose > 0 {
        conf.verbose = args.verbose as usize;
    }

    // Initialize logger
    let log_level = match conf.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    unsafe {
        env::set_var("RUST_LOG", log_level);
    }
    env_logger::init();

    info!("Starting client with configuration: {:?}", conf);

    // Determine number of workers
    let num_workers = if let Some(max_clients) = conf.max_clients {
        if max_clients > 0 {
            max_clients
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        }
    } else {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    };

    info!("Spawning {} workers", num_workers);

    let mut alert_txs: Vec<std::sync::mpsc::SyncSender<String>> = Vec::new();
    let mut worker_rxs: Vec<std::sync::mpsc::Receiver<String>> = Vec::new();
    for _ in 0..num_workers {
        let (tx, rx) = std::sync::mpsc::sync_channel(10000);
        alert_txs.push(tx);
        worker_rxs.push(rx);
    }

    // Channel for batches of events
    let (batch_tx, batch_rx) = mpsc::channel(100);
    let batch_rx = std::sync::Arc::new(tokio::sync::Mutex::new(batch_rx));

    // Initialize EventBatchQueue on stack
    let queue = queue::EventBatchQueue::new(0); // 0 second delta for immediate processing

    // Initialize Listener on stack
    let listener = listener::Listener::new(&conf.file);

    // Use scoped threads to share stack-allocated queue and listener
    let server = conf.server.clone();
    let port = conf.port;
    let insecure = conf.insecure;
    let batch_rx_clone = batch_rx.clone();
    tokio::spawn(async move {
        loop {
            let mut client = loop {
                match client::Client::new(&server, port, insecure).await {
                    Ok(c) => break c,
                    Err(e) => {
                        error!(
                            "Failed to create gRPC client: {}. Retrying in 2 seconds...",
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            };
            if let Err(e) = client.stream_data(batch_rx_clone.clone()).await {
                error!("gRPC streaming error: {}. Reconnecting...", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            } else {
                // Stream ended normally (server closed?)
                warn!("gRPC stream ended. Reconnecting...");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });

    std::thread::scope(|s| {
        // Spawn Workers
        for i in 0..num_workers {
            let worker_rx = worker_rxs.remove(0); // Take ownership of one receiver
            let queue_ref = &queue;
            let sensor_id = conf.sensor_id.clone(); // Clone sensor_id for each worker

            s.spawn(move || {
                info!("Worker {} started", i);
                for line in worker_rx {
                    // Deserialize JSON here using simd-json
                    // simd-json requires a mutable byte slice
                    let mut line_bytes = line.into_bytes();
                    let alert_result: Result<types::SuricataAlert, _> =
                        simd_json::from_slice(&mut line_bytes);

                    match alert_result {
                        Ok(mut alert) => {
                            alert.metadata.sensor_id = sensor_id.clone();
                            if let Some((mut event, metric)) =
                                processor::convert_suricata_alert_to_sensor_event(&alert)
                            {
                                event.metrics.push(metric);
                                queue_ref.add(event);
                            }
                        }
                        Err(e) => {
                            error!("Worker {}: Failed to parse JSON: {}", i, e);
                        }
                    }
                }
                info!("Worker {} stopped", i);
            });
        }

        // Spawn Listener
        let listener_ref = &listener;
        let alert_txs_clone = alert_txs.clone(); // Clone for the listener thread
        s.spawn(move || {
            if let Err(e) = listener_ref.start(alert_txs_clone) {
                error!("Listener error: {}", e);
            }
        });

        // Spawn Watcher
        let queue_ref = &queue;
        let batch_tx_clone = batch_tx.clone(); // Clone for the watcher thread
        s.spawn(move || {
            loop {
                // Poll frequently for high throughput
                std::thread::sleep(std::time::Duration::from_millis(10));
                let batch = queue_ref.process_batch();
                if !batch.is_empty() {
                    if let Err(e) = batch_tx_clone.blocking_send(batch) {
                        error!("Failed to send batch to gRPC client: {}", e);
                        break; // Exit loop if send fails (likely client disconnected)
                    }
                }
            }
        });

        // Spawn Metrics Updater
        let queue_ref = &queue;
        let listener_ref = &listener;
        s.spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            queue_ref.update_metrics();
            listener_ref.update_metrics();
        });

        // Spawn Metrics Logger
        let queue_ref = &queue;
        let listener_ref = &listener;
        s.spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                info!(
                    "Metrics: read_persec={} processed_persec={} batch_sent_persec={} total_processed={} total_sent={} queue_size={}",
                    listener_ref.get_event_read_per_second(),
                    queue_ref.get_event_processed_per_second(),
                    queue_ref.get_event_batch_sent_per_second(),
                    queue_ref.get_total_processed_events(),
                    queue_ref.get_total_sent_events(),
                    queue_ref.get_queue_size()
                );
            }
        });

        // The gRPC client needs to run concurrently with the scoped threads.
        // Since `std::thread::scope` blocks the main thread until all spawned threads join,
        // and the gRPC client is an async task that needs the Tokio runtime,
        // we cannot simply spawn it inside the `scope` block if we want it to run
        // concurrently with the blocking threads.
        // 2. The `scope` block will then run its blocking threads, while the Tokio runtime
        //    (managed by `#[tokio::main]`) continues to run the gRPC client task in the background.
        //    The main thread will block on `scope` until the blocking threads finish.
        //    This means the program will exit when the blocking threads finish, even if the gRPC client
        //    is still running. This is usually fine if the blocking threads are the primary producers.

        // Let's move the gRPC client spawn *before* the scope.
        // This requires `batch_rx` to be moved into the async block.
        // The `scope` will then block, and the gRPC client will run on the Tokio runtime.
        // The program will exit when the `scope` finishes and the main thread proceeds to `Ok(())`.
        // If the gRPC client is meant to be the "main" loop, then the `scope` should probably
        // not block the main thread, or the gRPC client should be spawned in a way that
        // it keeps the main thread alive.
        // For now, let's assume the blocking threads are the primary producers and the gRPC client
        // is a consumer that should run concurrently.
    });

    Ok(())
}
