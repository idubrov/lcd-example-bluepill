use stm32f103xx::SYST;

macro_rules! set_pin {
    ( $port:expr, $pin:expr, $flag:expr  ) => {
        let offset = if $flag { $pin } else { $pin + 16 };
        $port.bsrr.write(|w| unsafe { w.bits(1 << offset) });
    };
}

pub fn delay_us(syst: &SYST, delay: u32) {
    syst.set_reload(delay); // SysTick is 1/8 AHB (9Mhz)
    syst.clear_current();
    while !syst.has_wrapped() {}
}