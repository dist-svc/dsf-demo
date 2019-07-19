//#![feature(await_macro, async_await, futures_api)]

use std::io;

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

extern crate actix;
use actix::prelude::*;

use tokio::prelude::*;
use tokio::timer::Interval;

#[macro_use]
extern crate tokio_async_await;

extern crate dsf_core;
use dsf_core::types::Id;
use dsf_core::base::Body;
use dsf_core::api::*;

extern crate dsf_impl;
use dsf_impl::client::*;
use dsf_impl::rpc::{RequestKind, ResponseKind, ServiceCommands, ListOptions};

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

fn main() -> Result<(), io::Error> {
    // Fetch arguments
    let config = Config::from_args();
    let id = config.publisher_id;

    // Setup logging
    TermLogger::init(config.level, simplelog::Config::default()).unwrap();

    System::run(move || {
        // Create client connection
        let mut c = match Client::new(&config.daemon_socket) {
            Ok(c) => c,
            Err(e) => {
                error!("Error connecting to daemon on '{}': {:?}", &config.daemon_socket, e);
                System::current().stop();
                return
            }
        };

        actix::spawn(
            // Locate the service and join the client
            c.locate(&config.publisher_id).map(|s| (c, s) )       
            // Subscribe to future data
            .and_then(move |(mut c, s)| {
                // Subscribe to service and receive all future data
                c.subscribe(&s, SubscribeOptions::default()).map(|x| (c, s, x) )
            }).and_then(move |(mut _c, s, x)| {
                // Handle all incoming subscription responses
                x.for_each(move |msg| {
                    // Filter for data messages
                    let data = match msg {
                        ResponseKind::Data(data) => data,
                        _ => return Ok(())
                    };

                    // Output data
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

                    Ok(())
                })
            }).map_err(|e| {
                error!("DSF error: {:?}", e);
                System::current().stop();
            })
        )
    })
}
// Start sensor task
        
