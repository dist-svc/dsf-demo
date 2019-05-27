#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::{TermLogger, LevelFilter};

extern crate structopt;
use structopt::StructOpt;

extern crate serde;
extern crate serde_json;


extern crate futures;
use futures::prelude:*;

extern crate tokio;
use tokio::prelude:*;
use tokio::timer::Interval;

#[macro_use]
extern crate tokio_async_await;

extern crate dsf_core;
use dsf_core::types::Id;

extern crate dsf_impl;
use dsf_impl::client::Client;
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
    #[structopt(short = "i", long = "id")]
    /// Service ID for sensor data publishing
    id: Id,

    #[structopt(long = "i2c-device", default_value = "/dev/i2c-1")]
    /// Specify the I2C port for the sensor
    i2c_dev: String,

    #[structopt(long = "period", default_value = "1m")];
    /// Specify a period for sensor readings
    period: Duration,

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

impl From<bme280::Measurements> for Measurements {
    fn from(o: bme280::Measurements) -> Self {
        Self {
            temperature: o.temperature,
            pressure: o.pressure,
            humidity: o.humidity,
        }
    }
}

fn main() {
    // Fetch arguments
    let config = Config::from_args();

    // Setup logging
    TermLogger::init(opts.level, simplelog::Config::default()).unwrap();

    // Connect to sensor
    let i2c_bus = I2cdev::new(config.i2c_dev).expect("error connecting to i2c bus");
    let mut bme280 = BME280::new_primary(i2c_bus, Delay);

    bme280.init().expect("error initialising bme280");

    tokio::run_async( async {
        // Connect to daemon
        let c = await!(Client::new(config.daemon_socket));

        // Fetch service instance (managed by daemon)
        let s = await!(c.locate(id)).expect("error fetching service");

        // Check service is origin
        if !s.is_origin {
            error!("data can only be published using local services")
        }

        // Start sensor task
        Interval::new_interval(config.period)
            .for_each(move || {
                // Read sensor data
                let m: Measurements = bme280.measure().expect("error taking measurement");

                // Convert into JSON data
                let d = serde_json::to_string(&m);

                // Publish data
                await!(c.publish(&s, d.into_bytes()).expect("error publishing service data"));
            })

    });
}
