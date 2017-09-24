#![feature(const_fn)]
#![feature(used)]
#![feature(proc_macro)]
#![no_std]

extern crate stm32f103xx;
extern crate lcd;
extern crate cortex_m;
extern crate stm32_extras;

use core::fmt::Write;
use stm32f103xx::{SYST, GPIOA, GPIOB, RCC, FLASH};
use lcd::*;
use stm32_extras::GPIOExtras;

/// Delay for a given amount of microseconds. Should not be used for precise delays.
/// Assumes SYST ticks every microsecand and the reload value of 0xffffff (maximum).
/// `delay` must be less than 0x8000_0000 (SYST is only 24-bit)
pub fn delay_us(syst: &SYST, delay: u32) {
    // Essentialy, we do modulo 24-bit arithmetic.
    let stop_at: u32 = syst.get_current().wrapping_sub((delay * 9) - 1);
    // Run while `stop_at` is less than the counter value ("sign" bit of the difference is zero)
    // "sign" bit is 24th bit as SYST is 24-bit timer
    // Run while "(current - (start - delay)) | mod 0x800000 >= 0"
    while (syst.get_current().wrapping_sub(stop_at) & 0x00800000) == 0 { }
}

const RS: usize = 1; // PB1 is RS
const RW: usize = 10; // PB10 is RW
const E: usize = 11; // PB11 is E
const DATA: usize = 12; // PB12-PB15 is DB4-DB7


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


fn wait_condition<F>(syst: &SYST, f: F) -> bool
    where
        F: Fn() -> bool {
    syst.clear_current();
    while !f() {
        if syst.has_wrapped() {
            return false
        }
    }
    true
}


/// Enables `HSE` oscillator (assumes 8Mhz crystal).
/// Enables `PLL` with multiplier of 9 (72Mhz)
/// Sets up `SYSCLK` to use `PLL` as a source
/// Sets up `SysTick` to run at 1ms period.
pub fn setup(rcc: &RCC, syst: &SYST, flash: &FLASH) {
    if rcc.cr.read().pllrdy().is_locked() {
        panic!("PLL must be unlocked at this moment!");
    }

    // SysTick is AHB/8, which gives us 1Mhz
    syst.set_reload(50_000 - 1); // 50ms timeout ticks
    syst.enable_counter();

    // Use two wait states (48MHz < SYSCLK <= 72MHz)
    flash.acr.modify(|_, w| w.latency().two());

    // Start HSE
    rcc.cr.modify(|_, w| w.hseon().enabled()); // Enable HSE
    if !wait_condition(syst, || rcc.cr.read().hserdy().is_ready()) {
        panic!("HSE failed to start");
    }

    // Configure dividers
    rcc.cfgr.modify(|_, w| w
        .hpre().div1() // AHB clock prescaler
        .ppre1().div2() // APB low-speed prescaler
        .ppre2().div1() // APB high-speed prescaler
        .pllsrc().external() // Use HSE as source for PLL
        .pllxtpre().div1().pllmul().mul9() // /1*9 = 72Mhz
    );

    // Lock PLL
    rcc.cr.modify(|_, w| w.pllon().enabled());
    if !wait_condition(syst, || rcc.cr.read().pllrdy().is_locked()) {
        panic!("PLL failed to lock");
    }

    // Use PLL as a source for SYSCLK
    rcc.cfgr.modify(|_, w| w.sw().pll());
    if !wait_condition(syst, || rcc.cfgr.read().sws().is_pll()) {
        panic!("SYSCLK failed to switch to PLL");
    }

    // Setup SysTick to run at 1ms
    // SysTick is 1/8 AHB (9Mhz)
    syst.set_reload(9_000 - 1);
    syst.clear_current();
}


// Optional, if not implemented `lcd` library will use delays
/*#[cfg(feature = "input")]
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
}*/

fn main() {
    cortex_m::interrupt::free(
        |cs| {
            let syst = SYST.borrow(cs);
            let rcc = RCC.borrow(cs);
            let gpioa = GPIOA.borrow(cs);
            let gpiob = GPIOB.borrow(cs);
            let flash = FLASH.borrow(cs);
            run(&syst, &rcc, &gpioa, &gpiob, &flash);
        }
    );
}

fn bit(bit: bool) -> u8 {
    if bit { 1 } else { 0 }
}

fn run(syst: &SYST, rcc: &RCC, gpioa: &GPIOA, gpiob: &GPIOB, flash: &FLASH) {
    setup(rcc, syst, flash);
    // Used for delays
    // SysTick is 1/8 AHB (1Mhz with default clock settings)
    syst.enable_counter();
    syst.set_reload(0x00ffffff);

    // Setup GPIOB for LCD (all ports are in output mode)
    rcc.apb2enr.modify(|_, w| w.iopben().enabled());
    rcc.apb2enr.modify(|_, w| w.iopaen().enabled());

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
        /*display.position(0, 0);
        write!(&mut display, "Hello!").unwrap();
        delay_us(syst, 500_000);

        display.position(0, 0);
        write!(&mut display, "Bye!  ").unwrap();
        delay_us(syst, 500_000);*/

        display.position(0, 0);
        write!(&mut display, "{} {} {}", bit(gpioa.read_pin(1)), bit(gpioa.read_pin(2)), bit(gpioa.read_pin(3))).unwrap();
        display.position(0, 1);
        write!(&mut display, "{} {} {}", bit(gpioa.read_pin(5)), bit(gpioa.read_pin(6)), bit(gpioa.read_pin(7))).unwrap();
    }
}
