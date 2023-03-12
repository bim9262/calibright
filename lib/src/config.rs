use crate::consts::*;
use crate::errors::*;
use crate::util::*;

use std::collections::HashMap;

use serde::Deserialize;
use serde::Deserializer;
use smart_default::SmartDefault;

make_log_macro!(debug, "config");

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
struct UnresolvedDeviceConfig {
    #[serde(default, deserialize_with = "deserialize_root_scaling")]
    root_scaling: Option<f64>,

    ddcci_sleep_multiplier: Option<f64>,

    ddcci_max_tries_write_read: Option<u8>,

    #[serde(default, deserialize_with = "deserialize_calibration")]
    calibration: Option<[f64; 2]>,
}

fn deserialize_root_scaling<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let root_scaling = Option::<f64>::deserialize(deserializer)?;

    if let Some(root_scaling) = root_scaling {
        debug!("{:?}", root_scaling);

        if !ROOT_SCALDING_RANGE.contains(&root_scaling) {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Float(root_scaling),
                &"number in the range of 0.1 to 10.",
            ));
        }
    }

    Ok(root_scaling)
}

fn deserialize_calibration<'de, D>(deserializer: D) -> Result<Option<[f64; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let calibration = Option::<[f64; 2]>::deserialize(deserializer)?;
    if let Some(calibration) = calibration {
        debug!("{:?}", calibration);
        if calibration[0] > calibration[1] {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Other(format!("{calibration:?}").as_str()),
                &format!(
                    "Invalid scale parameters: {} > {}",
                    calibration[0], calibration[1]
                )
                .as_str(),
            ));
        }

        for val in calibration {
            if !CALIBRATION_RANGE.contains(&val) {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Float(val),
                    &"number in the range of 0.0 to 100.0",
                ));
            }
        }
    }
    Ok(calibration.map(|limits| limits.map(|val| val / 100.0)))
}

#[derive(Clone, Debug, SmartDefault)]
pub struct DeviceConfig {
    #[default(1.0)]
    pub root_scaling: f64,
    #[default(1.0)]
    pub ddcci_sleep_multiplier: f64,
    #[default(10)]
    pub ddcci_max_tries_write_read: u8,
    /// Calibration values are given as 0-100 in the config, but mapped to 0-1
    #[default([0.0, 1.0])]
    pub calibration: [f64; 2],
}

#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct UnresolvedCalibrightConfig {
    global: UnresolvedDeviceConfig,
    #[serde(flatten)]
    overrides: HashMap<String, UnresolvedDeviceConfig>,
}

#[derive(Clone)]
pub struct CalibrightConfig {
    global: DeviceConfig,
    overrides: HashMap<String, DeviceConfig>,
}

impl UnresolvedCalibrightConfig {
    fn resolve(&self, defaults: &DeviceConfig) -> CalibrightConfig {
        let global = DeviceConfig {
            root_scaling: self.global.root_scaling.unwrap_or(defaults.root_scaling),
            ddcci_sleep_multiplier: self
                .global
                .ddcci_sleep_multiplier
                .unwrap_or(defaults.ddcci_sleep_multiplier),
            ddcci_max_tries_write_read: self
                .global
                .ddcci_max_tries_write_read
                .unwrap_or(defaults.ddcci_max_tries_write_read),
            calibration: self.global.calibration.unwrap_or(defaults.calibration),
        };

        let mut resolved_overrides = HashMap::<String, DeviceConfig>::new();

        for (device_name, device_config) in &self.overrides {
            resolved_overrides.insert(
                device_name.to_owned(),
                DeviceConfig {
                    root_scaling: device_config.root_scaling.unwrap_or(global.root_scaling),
                    ddcci_sleep_multiplier: device_config
                        .ddcci_sleep_multiplier
                        .unwrap_or(global.ddcci_sleep_multiplier),
                    ddcci_max_tries_write_read: device_config
                        .ddcci_max_tries_write_read
                        .unwrap_or(global.ddcci_max_tries_write_read),
                    calibration: device_config.calibration.unwrap_or(global.calibration),
                },
            );
        }

        CalibrightConfig {
            global,
            overrides: resolved_overrides,
        }
    }
}

impl CalibrightConfig {
    pub async fn new() -> Result<Self> {
        CalibrightConfig::new_with_defaults(&DeviceConfig::default()).await
    }

    pub async fn new_with_defaults(defaults: &DeviceConfig) -> Result<Self> {
        if let Some(config_path) = find_file("config", None, Some("toml")) {
            deserialize_toml_file(config_path)
        } else {
            Ok(UnresolvedCalibrightConfig::default())
        }
        .map(|config| config.resolve(defaults))
    }

    pub(crate) fn get_device_config(&self, device_name: &String) -> DeviceConfig {
        debug!("{}", device_name);
        if let Some(device_config) = self.overrides.get(device_name) {
            debug!("{:?}", device_config);
            device_config.clone()
        } else {
            debug!("using global config");
            self.global.clone()
        }
    }
}
