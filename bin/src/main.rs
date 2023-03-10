use calibright::{CalibrightBuilder, CalibrightError};

use clap::{ArgGroup, Parser};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
            ArgGroup::new("action")
                .required(true)
                .args(["get", "set", "inc", "dec"]),
        ))]
struct Args {
    /// Regular expression for the devices to match
    #[arg(long, value_name = "regex", default_value_t = String::from("."))]
    device: String,

    /// Print out the current backlight brightness of each output with such a control.
    /// The brightness is represented as a percentage of the maximum brightness supported.
    #[arg(long)]
    get: bool,

    /// Sets each backlight brightness to the specified level.
    #[arg(long, value_name = "percent")]
    set: Option<f64>,

    /// Increases brightness by the specified amount.
    #[arg(long, value_name = "percent")]
    inc: Option<f64>,

    /// Decreases brightness by the specified amount.
    #[arg(long, value_name = "percent")]
    dec: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), CalibrightError> {
    env_logger::init();
    let args = Args::parse();

    let mut calibright = CalibrightBuilder::new()
        .with_device_regex(args.device.as_str())
        .build()
        .await?;

    if let Some(set) = args.set {
        calibright.set_brightness(set / 100.0).await?;
    } else {
        let brightness = calibright.get_brightness().await?;
        if args.get {
            println!("{:?}", (brightness * 100.0).round());
        } else if let Some(inc) = args.inc {
            calibright.set_brightness(brightness + inc / 100.0).await?;
        } else if let Some(dec) = args.dec {
            calibright.set_brightness(brightness - dec / 100.0).await?;
        }
    }

    Ok(())
}
