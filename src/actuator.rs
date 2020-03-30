//#![feature(await_macro, async_await, futures_api)]

use std::time::Duration;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::{TermLogger, LevelFilter};

extern crate structopt;
use structopt::StructOpt;

#[macro_use]
extern crate serde;
extern crate serde_json;

extern crate futures;

extern crate async_std;
use async_std::prelude::*;
use async_std::task;

extern crate dsf_core;
use dsf_core::types::Id;
use dsf_core::base::Body;

extern crate dsf_client;
use dsf_client::prelude::*;

extern crate dsf_rpc;
use dsf_rpc::{ResponseKind};

extern crate linux_embedded_hal as hal;
extern crate bme280;

#[derive(StructOpt)]
#[structopt(name = "DSF Demo Sensor")]
struct Config {
    #[structopt(short = "i", long = "publisher-id")]
    /// Service ID for data subscription
    publisher_id: Id,

    #[structopt(short = "d", long = "daemon-socket", default_value = "/tmp/dsf.sock")]
    /// Specify the socket to bind the DSF daemon
    daemon_socket: String,

    #[structopt(long = "log-level", default_value = "info")]
    /// Enable verbose logging
    level: LevelFilter,
}

#[derive(Serialize, Deserialize)]
struct Measurements {
    temperature: f32,
    pressure: f32,
    humidity: f32,
}

fn main() {
    // Fetch arguments
    let config = Config::from_args();

    // Setup logging
    TermLogger::init(config.level, simplelog::Config::default()).unwrap();

    // Create async task
    let r: Result<(), ClientError> = task::block_on(async {
        // Create client connection
        let mut client = Client::new(&config.daemon_socket, Duration::from_secs(3))?;

        // Locate the service and join the client
        let (service, _info) = client.locate(&config.publisher_id).await?;

        // Subscribe to service and receive all future data
        let mut sub = client.subscribe(&service, ()).await?;

        //let sub = Box::pin(sub);

        // Handle incoming responses
        while let Some(msg) = sub.next().await {
            // Filter for data messages
            let data = match msg {
                ResponseKind::Data(data) => data,
                _ => continue,
            };

            for d in &data {
                // Filter for cleartext body
                let b = match &d.body {
                    Body::Cleartext(t) => t,
                    _ => return Ok(())
                };

                // Convert into JSON data
                let m: Measurements = serde_json::from_slice(&b).unwrap();

                info!("measurement: {:.2}°C, {:.2} kPa, {:2} %RH", m.temperature,  m.pressure, m.humidity);
            }

        }

        Ok(())
    });

    if let Err(e) = r {
        error!("Error: {:?}", e);
    }
}

        
