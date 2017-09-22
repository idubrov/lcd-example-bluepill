#![feature(const_fn)]
#![feature(used)]
#![feature(proc_macro)]
#![no_std]

extern crate stm32f103xx;
extern crate lcd;
extern crate cortex_m;
extern crate stm32_extras;

use core::fmt::Write;
use stm32f103xx::{SYST, GPIOB, RCC};
use lcd::*;
use stm32_extras::GPIOExtras;

/// Delay for a given amount of microseconds. Should not be used for precise delays.
/// Assumes SYST ticks every microsecand and the reload value of 0xffffff (maximum).
/// `delay` must be less than 0x8000_0000 (SYST is only 24-bit)
pub fn delay_us(syst: &SYST, delay: u32) {
    // Essentialy, we do modulo 24-bit arithmetic.
    let stop_at: u32 = syst.get_current().wrapping_sub(delay - 1);
    // Run while `stop_at` is less than the counter value ("sign" bit of the difference is zero)
    // "sign" bit is 24th bit as SYST is 24-bit timer
    // Run while "(current - (start - delay)) | mod 0x800000 >= 0"
    while (syst.get_current().wrapping_sub(stop_at) & 0x00800000) == 0 { }
}

const RS: usize = 12; // PB12 is RS
const RW: usize = 13; // PB13 is RW
const E: usize = 14; // PB14 is E
const DATA: usize = 6; // PB6-PB9 is DB4-DB7


/// Binding of HD44780 instance to the real hardware
pub struct LcdHardware<'a> {
    syst: &'a SYST,
    gpiob: &'a GPIOB,
}

impl<'a> lcd::Hardware for LcdHardware<'a> {
    fn rs(&self, bit: bool) {
        self.gpiob.write_pin(RS, bit);
    }

    fn enable(&self, bit: bool) {
        self.gpiob.write_pin(E, bit);
    }

    fn data(&self, data: u8) {
        self.gpiob.write_pin_range(DATA, 4, u16::from(data));
    }
}

impl<'a> lcd::Delay for LcdHardware<'a> {
    fn delay_us(&self, delay_usec: u32) {
        delay_us(self.syst, delay_usec);
    }
}

// Optional, if not implemented `lcd` library will use delays
#[cfg(feature = "input")]
impl<'a> lcd::InputCapableHardware for LcdHardware<'a> {
    fn rw(&self, bit: bool) {
        if bit {
            // LCD has OD output, set all to '0' just to be sure.
            self.gpiob.write_pin_range(DATA, 4, 0);

            // Re-configure port for input
            for i in 0..4 {
                self.gpiob.pin_config(DATA + i).input().floating();
            }

            // Finally, set R/W to 1 (read)
            self.gpiob.write_pin(RW, true);
        } else {
            // First, set R/W to 0 (write mode)
            self.gpiob.write_pin(RW, false);

            // To be sure LCD is in read mode
            delay_us(self.syst, 1);

            // Re-configure port back to output
            for i in 0..4 {
                self.gpiob.pin_config(DATA + i).push_pull().output2();
            }
        }
    }

    fn read_data(&self) -> u8 {
        self.gpiob.read_pin_range(6, 4) as u8
    }
}

fn main() {
    cortex_m::interrupt::free(
        |cs| {
            let syst = SYST.borrow(cs);
            let rcc = RCC.borrow(cs);
            let gpiob = GPIOB.borrow(cs);
            run(&syst, &rcc, &gpiob);
        }
    );
}

fn run(syst: &SYST, rcc: &RCC, gpiob: &GPIOB) {
    // Used for delays
    // SysTick is 1/8 AHB (1Mhz with default clock settings)
    syst.enable_counter();
    syst.set_reload(0x00ffffff);

    // Setup GPIOB for LCD (all ports are in output mode)
    rcc.apb2enr.modify(|_, w| w.iopben().enabled());

    for i in 0..4 {
        gpiob.pin_config(DATA + i).push_pull().output2();
    }

    gpiob.pin_config(RS).push_pull().output2();
    gpiob.pin_config(RW).push_pull().output2();
    gpiob.pin_config(E).push_pull().output2();

    gpiob.write_pin(RS, false);
    gpiob.write_pin(RW, false);
    gpiob.write_pin(E, false);

    // Init display
    let mut display = Display::new(LcdHardware { syst, gpiob });
    display.init(FunctionLine::Line2, FunctionDots::Dots5x8);
    display.display(DisplayMode::DisplayOn, DisplayCursor::CursorOff, DisplayBlink::BlinkOff);

    // Print in loop
    loop {
        display.position(0, 0);
        write!(&mut display, "Hello!").unwrap();
        delay_us(syst, 500_000);

        display.position(0, 0);
        write!(&mut display, "Bye!  ").unwrap();
        delay_us(syst, 500_000);
    }
}
