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
use async_std::task;

extern crate dsf_core;
use dsf_core::types::{Id, DataKind};

extern crate dsf_client;
use dsf_client::prelude::*;

extern crate linux_embedded_hal as hal;
extern crate bme280;

use hal::{Delay, I2cdev};
use bme280::BME280;

extern crate humantime;
use humantime::{Duration as HumanDuration};


#[derive(StructOpt)]
#[structopt(name = "DSF Demo Sensor")]
struct Config {
    #[structopt(short = "i", long = "service-id")]
    /// Service ID for sensor data publishing
    service_id: Id,

    #[structopt(long = "i2c-device", default_value = "/dev/i2c-1")]
    /// Specify the I2C port for the sensor
    i2c_dev: String,


    #[structopt(long = "i2c-addr", default_value = "0x77")]
    /// Specify the I2C address for the sensor
    i2c_addr: u8,

    #[structopt(long = "period", default_value = "1m")]
    /// Specify a period for sensor readings
    period: HumanDuration,

    #[structopt(short = "d", long = "daemon-socket", default_value = "/tmp/dsf.sock")]
    /// Specify the socket to bind the DSF daemon
    daemon_socket: String,

    #[structopt(long = "log-level", default_value = "info")]
    /// Enable verbose logging
    level: LevelFilter,
}

#[derive(Debug, Serialize, Deserialize)]
struct Measurements {
    temperature: f32,
    pressure: f32,
    humidity: f32,
}

impl <E> From<bme280::Measurements<E>> for Measurements {
    fn from(o: bme280::Measurements<E>) -> Self {
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
    TermLogger::init(config.level, simplelog::Config::default()).unwrap();

    // Connect to sensor
    let i2c_bus = I2cdev::new(&config.i2c_dev).expect("error connecting to i2c bus");
    let mut bme280 = BME280::new(i2c_bus, config.i2c_addr, Delay);

    bme280.init().expect("error initialising bme280");

    let r: Result<(), ClientError> = task::block_on(async {
        info!("Connecting to client: {}", &config.daemon_socket);

        // Create client connection
        let mut client = Client::new(&config.daemon_socket, Duration::from_secs(3))?;

        // Locate the service
        let (service, _info) = client.locate(&config.service_id).await?;

        info!("Entering sensor loop");

        // Start sensor loop
        loop {

            // Read sensor data
            let m: Measurements = bme280.measure().unwrap().into();

            info!("Publishing measurement: {:?}", m);

            // Convert into JSON object
            let d = serde_json::to_string(&m).unwrap();

            // Publish data
            client.publish(&service, Some(DataKind::Generic.into()), Some(&d.into_bytes())).await?;

            // Wait for next tick
            task::sleep(*config.period).await;
        }
    });


    if let Err(e) = r {
        error!("Error: {:?}", e);
    }
}
