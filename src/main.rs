#![feature(const_fn)]
#![feature(used)]
#![feature(proc_macro)]
#![no_std]

extern crate stm32f103xx;
extern crate lcd;
extern crate cortex_m;

#[macro_use]
mod util;

use core::fmt::Write;
use stm32f103xx::{SYST, GPIOB, RCC};
use lcd::*;
use util::delay_us;

/// Binding of HD44780 instance to the real hardware
pub struct LcdHardware<'a> {
    syst: &'a SYST,
    gpiob: &'a GPIOB,
}

impl<'a> lcd::Hardware for LcdHardware<'a> {
    fn rs(&self, bit: bool) {
        set_pin!(self.gpiob, 12, bit);
    }

    fn enable(&self, bit: bool) {
        set_pin!(self.gpiob, 14, bit);
    }

    fn data(&self, data: u8) {
        let bits = u32::from(data & 0b1111) | // Set '1's
            (u32::from(!data & 0b1111) << 16); // Clear '0's
        self.gpiob.bsrr.write(|w| unsafe { w.bits(bits << 6) });
    }
}

impl<'a> lcd::Delay for LcdHardware<'a> {
    fn delay_us(&self, delay_usec: u32) {
        delay_us(self.syst, delay_usec);
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
    // Used for delay
    syst.enable_counter();

    // Setup GPIOB for LCD (all ports are in output mode)
    rcc.apb2enr.modify(|_, w| w.iopben().enabled());

    gpiob.crl.modify(|_, w| w
        .cnf6().push().mode6().output() // PB6 is DB4
        .cnf7().push().mode7().output()); // PB7 is DB5
    gpiob.crh.modify(|_, w| w
        .cnf12().push().mode12().output() // PB12 is RS
        .cnf13().push().mode13().output() // PB13 is R/W
        .cnf14().push().mode14().output() // PB14 is E
        .cnf8().push().mode8().output() // PB8 is DB6
        .cnf9().push().mode9().output()); // PB9 is DB7

    // R/W is always 0 -- we don't use wait flag
    set_pin!(gpiob, 13, false);

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
