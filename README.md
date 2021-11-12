# floatpack
Bitpacking with SIMD for `Decimal` from the `rust_decimal` crate. 

# In a nutshell
The algorithm:
1. Each `Decimal` value is serialized into its components (= 4 x `u32`)
2. The resulting 4 component streams are individually compressed by storing their cumulative difference (XOR)
3. The 4 compressed component streams are then bit-packed

The idea is that this should get good compression rates with little computational complexity, especially for contiguous data. In timeseries data, usually:
* one datapoint only differs slightly from the next one in the series and
* you have a lot of datapoints

This represents a highly specific use-case. If you are not dealing with timeseries data other compression algorithms are probably more suitable.

# Usage example
``` rust
use floatpack::{pack, unpack};
use rust_decimal_macros::*;

let values = vec![dec!(1.0), dec!(2.0), dec!(3.0)];
assert_eq!(values, unpack(&pack(&values[..])));
```
