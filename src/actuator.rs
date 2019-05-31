//#![feature(await_macro, async_await, futures_api)]

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
use futures::prelude::*;

extern crate tokio;
use tokio::prelude::*;
use tokio::timer::Interval;

#[macro_use]
extern crate tokio_async_await;

extern crate dsf_core;
use dsf_core::types::Id;
use dsf_core::api::*;

extern crate dsf_impl;
use dsf_impl::client::*;
use dsf_impl::rpc::{RequestKind, ServiceCommands, ListOptions};

extern crate linux_embedded_hal as hal;
extern crate bme280;

use hal::{Delay, I2cdev};
use bme280::BME280;

extern crate humantime;
use humantime::Duration;

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
    let id = config.publisher_id;

    // Setup logging
    TermLogger::init(config.level, simplelog::Config::default()).unwrap();

    tokio::run( future::lazy(move || {
        // Connect to daemon and locate the service
        Client::new(&config.daemon_socket)
            .map_err(|e| panic!(e) )
            .and_then(move |mut c| c.locate(&id).map(|s| (c, s) ))
            .and_then(move |(mut c, s)| {
                // Subscribe to service and receive all future data
                c.subscribe(&s, SubscribeOptions::default())
                .for_each(move |data| {
                    // Convert into JSON data
                    let m: Measurements = serde_json::from_slice(&data).unwrap();

                    info!("temperature: {:.2}°C", m.temperature);
                    info!("pressure: {:.2} kPa", m.pressure);
                    info!("humidity: {:2} %RH", m.humidity);

                    Ok(())
                })
            })
    }).map_err(|e| panic!(e) ));
}
// Start sensor task
        
