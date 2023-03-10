use std::ops::RangeInclusive;

/// Location of backlight devices
pub const DEVICES_PATH: &str = "/sys/class/backlight";

/// Filename for device's max brightness
pub const FILE_MAX_BRIGHTNESS: &str = "max_brightness";

/// Filename for current brightness.
pub const FILE_BRIGHTNESS: &str = "actual_brightness";

/// amdgpu drivers set the actual_brightness in a different scale than
/// [0, max_brightness], so we have to use the 'brightness' file instead.
/// This may be fixed in the new 5.7 kernel?
pub const FILE_BRIGHTNESS_AMD: &str = "brightness";

/// set the requested brightness level
pub const FILE_BRIGHTNESS_WRITE: &str = "brightness";

/// Range of valid values for `root_scaling`
pub const ROOT_SCALDING_RANGE: RangeInclusive<f64> = 0.1..=10.;

/// Range of valid values for `Calibration`
pub const CALIBRATION_RANGE: RangeInclusive<f64> = 0.0..=100.;
