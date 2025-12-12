use crate::pb::sensor_service_client::SensorServiceClient;
use crate::pb::SensorEvent;
use log::{error, info};
use tokio::sync::mpsc;
use tonic::transport::{Channel, ClientTlsConfig};

pub struct Client {
    client: SensorServiceClient<Channel>,
}

impl Client {
    pub async fn new(
        host: &str,
        port: u16,
        insecure: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("http://{}:{}", host, port);
        let endpoint = Channel::from_shared(addr)?;

        let channel = if insecure {
            endpoint.connect().await?
        } else {
            let tls = ClientTlsConfig::new().domain_name(host);
            endpoint.tls_config(tls)?.connect().await?
        };

        let client = SensorServiceClient::new(channel);
        info!("Connected to gRPC server at {}:{}", host, port);

        Ok(Self { client })
    }

    pub async fn stream_data(
        &mut self,
        rx: std::sync::Arc<tokio::sync::Mutex<mpsc::Receiver<Vec<SensorEvent>>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Flatten the stream of batches into a stream of individual events
        let stream = async_stream::stream! {
            info!("Starting gRPC request stream");
            loop {
                let batch = {
                    let mut rx_guard = rx.lock().await;
                    rx_guard.recv().await
                };
                match batch {
                    Some(batch) => {
                        info!("Sending batch of {} events", batch.len());
                        for event in batch {
                            yield event;
                        }
                    }
                    None => break,
                }
            }
            info!("gRPC request stream ended");
        };

        let request = tonic::Request::new(stream);

        match self.client.stream_data(request).await {
            Ok(_) => {
                info!("Stream completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Stream failed: {}", e);
                Err(Box::new(e))
            }
        }
    }
}
