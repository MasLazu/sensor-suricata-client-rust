use log::{error, info, warn};
use std::fs;
use std::io::BufReader;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

use std::sync::atomic::{AtomicI64, Ordering};

pub struct Listener {
    socket_path: String,
    // Metrics
    read_this_sec: AtomicI64,
    latest_read_per_sec: AtomicI64,
}

impl Listener {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            read_this_sec: AtomicI64::new(0),
            latest_read_per_sec: AtomicI64::new(0),
        }
    }

    // This function is now blocking and should be run in a separate thread or spawn_blocking
    pub fn start(
        &self,
        txs: Vec<std::sync::mpsc::SyncSender<String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Remove existing socket file if it exists
        if Path::new(&self.socket_path).exists() {
            fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Listening on {}", self.socket_path);

        // Set socket permissions
        if let Ok(metadata) = fs::metadata(&self.socket_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o666);
            if let Err(e) = fs::set_permissions(&self.socket_path, perms) {
                warn!("Failed to set socket permissions: {}", e);
            }
        }

        // Accept connections (we expect only one from Suricata)
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    info!("Accepted connection from Suricata");
                    let reader = BufReader::new(stream);

                    // Read line by line
                    use std::io::BufRead;
                    let lines_iter = reader.lines();

                    let num_workers = txs.len();
                    let mut counter = 0;

                    for line in lines_iter {
                        match line {
                            Ok(line_content) => {
                                let idx = counter % num_workers;
                                // Use send since we are using std::sync::mpsc
                                if let Err(e) = txs[idx].send(line_content) {
                                    error!("Failed to send raw line to worker {}: {}", idx, e);
                                    break;
                                }
                                counter += 1;
                                self.read_this_sec.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(e) => {
                                error!("Error reading line: {}", e);
                                // Continue processing other events
                            }
                        }
                    }
                    info!("Connection closed");
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn get_event_read_per_second(&self) -> i64 {
        self.latest_read_per_sec.load(Ordering::Relaxed)
    }

    pub fn update_metrics(&self) {
        let count = self.read_this_sec.swap(0, Ordering::Relaxed);
        self.latest_read_per_sec.store(count, Ordering::Relaxed);
    }
}
