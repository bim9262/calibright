use crate::config::DeviceConfig;
use crate::consts::*;
use crate::errors::*;
use crate::util::*;

use std::cmp::max;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
#[cfg(feature = "watch")]
use std::time::Instant;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use zbus::Connection;

make_log_macro!(debug, "device");

#[zbus::dbus_proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait Session {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}

/// Represents a physical backlight device whose brightness level can be queried.
#[derive(Clone)]
pub struct Device {
    pub device_name: OsString,
    pub read_brightness_file: PathBuf,
    write_brightness_file: PathBuf,
    raw_brightness: u32,
    max_brightness: u32,
    dbus_proxy: SessionProxy<'static>,
    config: DeviceConfig,
    #[cfg(feature = "watch")]
    updated_at: Instant,
}

impl Device {
    pub async fn new(device_name: &String, config: DeviceConfig) -> Result<Self> {
        let device_path = PathBuf::from(DEVICES_PATH).join(device_name);

        let dbus_conn = Connection::system()
            .await
            .error("Failed to open DBus session connection")?;

        let mut s = Self {
            read_brightness_file: device_path.join({
                if device_path.ends_with("amdgpu_bl0") {
                    FILE_BRIGHTNESS_AMD
                } else {
                    FILE_BRIGHTNESS
                }
            }),
            write_brightness_file: device_path.join(FILE_BRIGHTNESS_WRITE),
            device_name: device_name.into(),
            raw_brightness: 0,
            max_brightness: 0,
            dbus_proxy: SessionProxy::new(&dbus_conn)
                .await
                .error("Failed to create SessionProxy")?,
            config,
            #[cfg(feature = "watch")]
            updated_at: Instant::now(),
        };
        s.raw_brightness = s.read_brightness_raw(&s.read_brightness_file).await?;
        s.max_brightness = s
            .read_brightness_raw(&device_path.join(FILE_MAX_BRIGHTNESS))
            .await?;
        Ok(s)
    }

    /// Read a brightness value from the given path.
    async fn read_brightness_raw(&self, device_file: &Path) -> Result<u32> {
        let val = match read_file(device_file).await {
            Ok(v) => Ok(v),
            Err(_) => {
                for i in 1..self.config.ddcci_max_tries_write_read {
                    debug!("retry {i} reading brightness");
                    // See https://glenwing.github.io/docs/VESA-DDCCI-1.1.pdf
                    // Section 4.3 for timing explanation
                    sleep(Duration::from_millis(
                        (40.0 * self.config.ddcci_sleep_multiplier).round() as u64,
                    ))
                    .await;
                    if let Ok(val) = read_file(device_file).await {
                        return val
                            .parse()
                            .error("Failed to read value from brightness file");
                    }
                }
                Err(Error::new(
                    "Failed to read brightness file, check your ddcci settings",
                ))
            }
        };
        val.error("Failed to read brightness file")?
            .parse()
            .error("Failed to read value from brightness file")
    }

    /// Query the brightness value for this backlight device, as a percent (0.0..=1.0).
    pub async fn get_brightness(&mut self) -> Result<f64> {
        self.raw_brightness = self.read_brightness_raw(&self.read_brightness_file).await?;

        let brightness_ratio = (self.raw_brightness as f64 / self.max_brightness as f64)
            .powf(self.config.root_scaling.recip());

        scale_to_clamped_absolute(
            brightness_ratio,
            self.config.calibration[0],
            self.config.calibration[1],
        )
    }

    /// Set the brightness value for this backlight device, as a percent (0.0..=1.0).
    pub async fn set_brightness(&mut self, value: f64) -> Result<()> {
        let value = scale_to_clamped_relative(
            value,
            self.config.calibration[0],
            self.config.calibration[1],
        )?;
        let ratio = value.powf(self.config.root_scaling);
        self.raw_brightness = max(1, (ratio * (self.max_brightness as f64)).round() as u32);
        match self
            .dbus_proxy
            .set_brightness(
                "backlight",
                &self.device_name.to_string_lossy(),
                self.raw_brightness,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!("{}", e.to_string());
                // Fall back to writing to sysfs brightness file
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&self.write_brightness_file)
                    .await
                    .error("Could not open brightness file to write")?;
                file.write_all(self.raw_brightness.to_string().as_bytes())
                    .await
                    .error("Could not write sysfs brightness")
            }
        }
        .map(|_| {
            #[cfg(feature = "watch")]
            {
                self.updated_at = Instant::now();
            }
        })
    }

    #[cfg(feature = "watch")]
    pub fn get_last_set_ago(&self) -> Duration {
        self.updated_at.elapsed()
    }
}
