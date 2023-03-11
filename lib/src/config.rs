use crate::consts::*;
use crate::errors::*;
use crate::util::*;

use std::collections::HashMap;

use serde::Deserialize;
use serde::Deserializer;
use smart_default::SmartDefault;

make_log_macro!(debug, "config");

#[derive(Deserialize, Clone, Debug, SmartDefault)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct DeviceConfig {
    #[default(1.0)]
    #[serde(deserialize_with = "deserialize_root_scaling")]
    pub root_scaling: f64,

    #[default(1.0)]
    pub ddcci_sleep_multiplier: f64,

    #[default(10)]
    pub ddcci_max_tries_write_read: u8,

    /// Calibration values are given as 0-100 in the config, but mapped to 0-1
    #[default([0.0, 1.0])]
    #[serde(deserialize_with = "deserialize_calibration")]
    pub calibration: [f64; 2],
}

fn deserialize_root_scaling<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let root_scaling = Deserialize::deserialize(deserializer)?;
    debug!("{:?}", root_scaling);

    if !ROOT_SCALDING_RANGE.contains(&root_scaling) {
        return Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Float(root_scaling),
            &"number in the range of 0.1 to 10.",
        ));
    }

    Ok(root_scaling)
}

fn deserialize_calibration<'de, D>(deserializer: D) -> Result<[f64; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let calibration: [f64; 2] = Deserialize::deserialize(deserializer)?;
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
    Ok(calibration.map(|val| val / 100.0))
}

#[derive(Deserialize, Clone, SmartDefault)]
#[serde(default)]
pub struct CalibrightConfig {
    global: DeviceConfig,
    #[serde(flatten)]
    overrides: HashMap<String, DeviceConfig>,
}

impl CalibrightConfig {
    pub async fn new(// default_root_scaling: Option<f64>,
        // default_ddcci_sleep_multiplier: Option<f64>,
        // default_ddcci_max_tries_write_read: Option<u8>,
    ) -> Result<Self> {
        if let Some(config_path) = find_file("config", None, Some("toml")) {
            deserialize_toml_file(config_path)
        } else {
            Ok(CalibrightConfig::default())
        }
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
