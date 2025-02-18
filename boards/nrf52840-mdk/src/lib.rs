#![no_std]

pub use nrf52840_hal as hal;
use hal::gpio::{p0, p1, Floating, Input};
pub use hal::nrf52840_pac as pac;

/// Exports traits that are usually needed when using this crate
pub mod prelude {
    pub use nrf52840_hal::prelude::*;
}

macro_rules! define_pins {
    ($(#[$topattr:meta])* struct $Type:ident,
    p0: {
     $( $(#[$attr:meta])* pin $name:ident = $pin_ident:ident : $pin_type:ident),+ ,
    },
    p1: {
     $( $(#[$attr1:meta])* pin $name1:ident = $pin_ident1:ident: $pin_type1:ident),+ ,
    }) => {

$(#[$topattr])*
pub struct $Type {
    $($(#[$attr])* pub $name: p0:: $pin_type <Input<Floating>>,)+
    $($(#[$attr1])* pub $name1: p1:: $pin_type1 <Input<Floating>>,)+
}

impl $Type {
    /// Returns the pins for the device
    pub fn new(pins0: p0::Parts, pins1: p1::Parts) -> Self {
        $Type {
            $($name: pins0.$pin_ident, )+
            $($name1: pins1.$pin_ident1, )+
        }
    }
}
}}

define_pins!(
    /// Maps the pins to the names printed on the device
    struct Pins,
    p0: {
        /// Uart RXD
        pin rxd = p0_19: P0_19,
        /// Uart TXD
        pin txd = p0_20: P0_20,

        pin p6 = p0_06: P0_06,
        pin p7 = p0_07: P0_07,
        pin p8 = p0_08: P0_08,
        pin p11 = p0_11: P0_11,
        pin p12 = p0_12: P0_12,
        pin p13 = p0_13: P0_13,
        pin p14 = p0_14: P0_14,
        pin p15 = p0_15: P0_15,
        pin p16 = p0_16: P0_16,
        pin p17 = p0_17: P0_17,
        pin p21 = p0_21: P0_21,
        pin p25 = p0_25: P0_25,
        pin p26 = p0_26: P0_26,
        pin p27 = p0_27: P0_27,


        pin ain0 = p0_02: P0_02,
        pin ain1 = p0_03: P0_03,
        pin ain2 = p0_04: P0_04,
        pin ain3 = p0_05: P0_05,
        pin ain4 = p0_28: P0_28,
        pin ain5 = p0_29: P0_29,
        pin ain6 = p0_30: P0_30,
        pin ain7 = p0_31: P0_31,

        pin nfc1 = p0_09: P0_09,
        pin nfc2 = p0_10: P0_10,

        pin red_led = p0_23: P0_23,
        pin green_led = p0_22: P0_22,
        pin blue_led = p0_24: P0_24,
    },
    p1: {
        pin button = p1_00: P1_00,

        /// ~RESET line to the QSPI flash
        pin qspi_reset = p1_01: P1_01,
        /// ~WP Write protect pin on the QSPI flash.
        pin qspi_wp = p1_02: P1_02,
        /// SPI SCLK for QSPI flash
        pin qspi_sclk = p1_03: P1_03,
        /// SPI MISO for QSPI flash
        pin qspi_miso = p1_04: P1_04,
        /// SPI MOSI for QSPI flash
        pin qspi_mosi = p1_05: P1_05,
        /// ~CS for the QSPI flash
        pin qspi_cs = p1_06: P1_06,
    }
);
