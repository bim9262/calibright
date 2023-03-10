#![warn(clippy::match_same_arms)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::unnecessary_wraps)]

#[macro_use]
mod util;
mod config;
mod consts;
mod device;
mod errors;
#[cfg(feature = "watch")]
mod watcher;

use std::collections::HashMap;
use std::ffi::OsString;

use futures::future::join_all;
use regex::Regex;
use tokio::fs::read_dir;

pub use crate::config::Config;
use crate::consts::*;
use crate::device::Device;
pub use crate::errors::Error;
use crate::errors::*;
use crate::util::*;
#[cfg(feature = "watch")]
use crate::watcher::*;

make_log_macro!(debug, "calibright");

pub struct CalibrightBuilder<'a> {
    device_regex: &'a str,
    config: Option<Config>,
    #[cfg(feature = "watch")]
    poll_interval: Duration,
}

impl<'a> Default for CalibrightBuilder<'a> {
    fn default() -> Self {
        Self {
            device_regex: ".",
            config: None,
            #[cfg(feature = "watch")]
            poll_interval: Duration::from_secs(2),
        }
    }
}

impl<'a> CalibrightBuilder<'a> {
    pub fn new() -> Self {
        CalibrightBuilder::default()
    }

    pub fn with_device_regex(mut self, device_regex: &'a str) -> Self {
        self.device_regex = device_regex;
        self
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    #[cfg(feature = "watch")]
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub async fn build(self) -> Result<Calibright> {
        let config = match self.config {
            Some(config) => config,
            None => Config::new().await?,
        };

        Calibright::new(
            Regex::new(self.device_regex).error("Illegal regex")?,
            config,
            #[cfg(feature = "watch")]
            self.poll_interval,
        )
        .await
    }
}

#[cfg(not(feature = "watch"))]
pub struct Calibright {
    devices: HashMap<OsString, Device>,
}

#[cfg(feature = "watch")]
pub struct Calibright {
    devices: HashMap<OsString, Device>,
    device_regex: Regex,
    config: CalibrightConfig,
    _poll_watcher: PollWatcher,
    inotify_watcher: INotifyWatcher,
    rx: Receiver<notify::Result<notify::Event>>,
    poll_interval: Duration,
}

impl Calibright {
    pub(crate) async fn new(
        device_regex: Regex,
        config: Config,
        #[cfg(feature = "watch")] poll_interval: Duration,
    ) -> Result<Self> {
        let mut sysfs_paths = read_dir(DEVICES_PATH)
            .await
            .error("Failed to read backlight device directory")?;

        let mut device_names = Vec::new();
        while let Some(sysfs_path) = sysfs_paths
            .next_entry()
            .await
            .error("No backlight devices found")?
        {
            let device_name = sysfs_path.file_name();
            if device_regex.is_match(&device_name.to_string_lossy()) {
                debug!(
                    "{:?} matched {}",
                    device_name.to_string_lossy().to_string(),
                    device_regex.as_str()
                );

                device_names.push(device_name.to_string_lossy().to_string());
            }
        }

        let mut device_map = HashMap::new();
        let device_list =
            join_all(device_names.iter().map(|device_name| {
                Device::new(device_name, config.get_device_config(device_name))
            }))
            .await;
        let device_list = device_list.iter().filter_map(|device| match device {
            Ok(device) => Some(device.to_owned()),
            Err(e) => {
                debug!("{e}");
                None
            }
        });

        #[cfg(not(feature = "watch"))]
        {
            for device in device_list {
                device_map.insert(device.device_name.clone(), device);
            }

            Ok(Calibright {
                devices: device_map,
            })
        }

        #[cfg(feature = "watch")]
        {
            let (_poll_watcher, mut inotify_watcher, rx) =
                pseudo_fs_watcher(DEVICES_PATH, poll_interval)?;

            for device in device_list {
                let watch_path = device.read_brightness_file.to_path_buf();
                inotify_watcher.watch(&watch_path, notify::RecursiveMode::NonRecursive)?;
                device_map.insert(device.device_name.clone(), device);
            }

            Ok(Calibright {
                devices: device_map,
                device_regex,
                config,
                _poll_watcher,
                inotify_watcher,
                rx,
                poll_interval,
            })
        }
    }

    #[cfg(feature = "watch")]
    pub async fn next(&mut self) -> Result<()> {
        use futures::StreamExt;
        use std::path::{Path, PathBuf};

        while let Some(res) = self.rx.next().await {
            let mut change_occurred = false;
            let event = res?;
            debug!("{:?}", event);
            let depth1_paths: Vec<&PathBuf> = event
                .paths
                .iter()
                .filter(|&p| p.parent() == Some(Path::new(DEVICES_PATH)))
                .collect();
            let brightness_paths: Vec<&PathBuf> = event
                .paths
                .iter()
                .filter(|&p| p.ends_with(FILE_BRIGHTNESS) || p.ends_with(FILE_BRIGHTNESS_AMD))
                .collect();
            if event.kind.is_create() && !depth1_paths.is_empty() {
                for path in depth1_paths {
                    if let Some(file_name) = path.file_name() {
                        let device_name = file_name.to_string_lossy().to_string();
                        debug!("New device {:?}", device_name);
                        if self.devices.contains_key(file_name) {
                            // We already know about this device, so no need to create a new `Device`
                            debug!("New device {:?}, already known", path);
                            continue;
                        }
                        if self.device_regex.is_match(&device_name) {
                            debug!("{:?} matched {}", device_name, self.device_regex.as_str());
                            let new_device = Device::new(
                                &device_name,
                                self.config.get_device_config(&device_name),
                            )
                            .await?;
                            let watch_path = new_device.read_brightness_file.clone();
                            self.inotify_watcher
                                .watch(&watch_path, notify::RecursiveMode::NonRecursive)?;
                            self.devices
                                .insert(new_device.device_name.clone(), new_device);
                            change_occurred = true;
                        }
                    }
                }
            } else if event.kind.is_remove() && !depth1_paths.is_empty() {
                for path in depth1_paths {
                    if let Some(file_name) = path.file_name() {
                        debug!("Remove {}", path.display());
                        if let Some(old_device) = self.devices.remove(file_name) {
                            debug!("Removed {}", old_device.read_brightness_file.display());
                            self.inotify_watcher
                                .unwatch(&old_device.read_brightness_file)?;
                            change_occurred = true;
                        }
                    }
                }
            } else if event.kind.is_modify() && !brightness_paths.is_empty() {
                for brightness_path in brightness_paths {
                    if let Some(path) = brightness_path.parent() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(device) = self.devices.get(file_name) {
                                if device.get_last_set_ago() > self.poll_interval {
                                    change_occurred = true;
                                }
                            }
                        }
                    }
                }
            }
            if change_occurred {
                return Ok(());
            }
        }
        Err(Error::new("Nothing to watch"))
    }

    pub async fn get_brightness(&mut self) -> Result<f64> {
        let brightnesses = join_all_accept_single_ok(
            self.devices
                .iter_mut()
                .map(|(_, device)| device.get_brightness()),
        )
        .await
        .error("No backlight devices found")?;

        Ok(brightnesses.iter().sum::<f64>() / (brightnesses.len() as f64))
    }

    pub async fn set_brightness(&mut self, brightness: f64) -> Result<()> {
        join_all_accept_single_ok(
            self.devices
                .iter_mut()
                .map(|(_, device)| device.set_brightness(brightness)),
        )
        .await
        .error("No backlight devices found")?;

        Ok(())
    }
}
