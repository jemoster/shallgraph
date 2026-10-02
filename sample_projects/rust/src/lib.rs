//! Tiny temperature converter used as a shallgraph tracing example.
//!
//! Tag implementation with `#[shallgraph::implements]` and tests with
//! `#[shallgraph::verifies]` so `shallgraph html` and `shallgraph markdown` can list source links.

extern crate shallgraph_macros as shallgraph;

/// Convert Celsius to Fahrenheit: `F = C * 9 / 5 + 32`.
#[shallgraph::implements("TEMP-001")]
pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

/// Convert Fahrenheit to Celsius: `C = (F - 32) * 5 / 9`.
#[shallgraph::implements("TEMP-002")]
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[shallgraph::verifies("TEMP-001")]
    #[test]
    fn converts_freezing_and_boiling_from_celsius() {
        assert_eq!(celsius_to_fahrenheit(0.0), 32.0);
        assert_eq!(celsius_to_fahrenheit(100.0), 212.0);
    }

    #[shallgraph::verifies("TEMP-002")]
    #[test]
    fn converts_freezing_and_boiling_from_fahrenheit() {
        assert_eq!(fahrenheit_to_celsius(32.0), 0.0);
        assert_eq!(fahrenheit_to_celsius(212.0), 100.0);
    }

    #[shallgraph::verifies("TEMP-001", "TEMP-002")]
    #[test]
    fn round_trips_freezing_point() {
        assert_eq!(fahrenheit_to_celsius(celsius_to_fahrenheit(0.0)), 0.0);
        assert_eq!(celsius_to_fahrenheit(fahrenheit_to_celsius(32.0)), 32.0);
    }
}
