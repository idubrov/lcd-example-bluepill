# lcd-example-bluepill

Minimal example of using [`lcd`](crates.io/crates/lcd) module on STM32 "Blue Pill" development board.

Control pins should be connected as following:
 * RS should be connected to PB12
 * R/W should be connected to PB13
 * E should be connected to PB14
 
Data pins should be connected (only high 4 pins are used):
 * DB4 should be connected to PB6
 * DB5 should be connected to PB7
 * DB6 should be connected to PB8
 * DB7 should be connected to PB9 

Run `make program` to build and program (assumes ST-LINK v2).