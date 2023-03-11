# calibright

### Docs

https://bmalyn.com/calibright

### Usage

You can create a config file at `$XDG_CONFIG_HOME/calibright/config.toml` with a `[global]` section as well as separate sections for each display.

All of the sections allow the same parameters:

Key                          | Value                                                                                             | Default
-----------------------------|---------------------------------------------------------------------------------------------------|---------
`root_scaling`               | Scaling exponent reciprocal (ie. root) Allows values from `0.1` to `10.0`                         | `1.0`
`ddcci_sleep_multiplier`     | See [ddcutil documentation](https://www.ddcutil.com/performance_options/#option-sleep-multiplier) | `1.0`
`ddcci_max_tries_write_read` | The maximum number of times to attempt writing to  or reading from a ddcci monitor                | `10`
`calibration`                | A pair of floats representing the the min and max brightness                                      | `[0.0, 100.0]`


A simple example config could look like:

```toml
[global]
ddcci_sleep_multiplier = 0.1

[ddcci6]
calibration = 90

[ddcci7]
calibration = [10, 80]
```