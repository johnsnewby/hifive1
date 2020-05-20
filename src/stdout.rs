//! Stdout based on the UART hooked up to FTDI or J-Link

use core::fmt;
use e310x_hal::{
    clock::Clocks,
    e310x::UART0,
    gpio::gpio0::{Pin16, Pin17},
    prelude::*,
    serial::{Rx, Serial, Tx},
    time::Bps,
};
use nb::block;
use riscv::interrupt;

static mut STDOUT: Option<SerialWrapper> = None;

struct SerialWrapper(Tx<UART0>, Rx<UART0>);

impl core::fmt::Write for SerialWrapper {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.as_bytes() {
            if *byte == '\n' as u8 {
                let res = block!(self.0.write('\r' as u8));

                if res.is_err() {
                    return Err(::core::fmt::Error);
                }
            }

            let res = block!(self.0.write(*byte));

            if res.is_err() {
                return Err(::core::fmt::Error);
            }
        }
        Ok(())
    }
}

/// Configures stdout
pub fn configure<X, Y>(uart: UART0, tx: Pin17<X>, rx: Pin16<Y>, baud_rate: Bps, clocks: Clocks) {
    let tx = tx.into_iof0();
    let rx = rx.into_iof0();
    let serial = Serial::new(uart, (tx, rx), baud_rate, clocks);
    let (tx, rx) = serial.split();

    interrupt::free(|_| unsafe {
        STDOUT.replace(SerialWrapper(tx, rx));
    });
}

/// reads bytes into a buffer
pub fn read(b: &mut [u8]) -> usize {
    let stdout: &mut SerialWrapper = unsafe {
        match STDOUT.as_mut() {
            Some(x) => x,
            None => return 0,
        }
    };
    let len = b.len();
    let mut idx = 0;
    while idx < len {
        let c = match block!(stdout.1.read()) {
            Ok(x) => x,
            Err(_) => break,
        };
        stdout.write_char(c as char);
        b[idx] = c;
        idx = idx + 1;
        if c == '\n' as u8 || c == '\r' as u8 {
            break;
        }
    }
    idx
}

/// reads bytes into a buffer
pub fn read_noblock(buf: &mut [u8], len: &mut usize) -> usize {
    let stdout: &mut SerialWrapper = unsafe {
        match STDOUT.as_mut() {
            Some(x) => x,
            None => return 0,
        }
    };
    let mut read: usize = 0;
    loop {
        if *len == buf.len() {
            break;
        }
        let c = match stdout.1.read() {
            Ok(x) => x,
            Err(_) => break,
        };
        read += 1;
        stdout.write_char(c as char);
        buf[*len] = c;
        *len += 1;
    }
    read
}

/// Writes string to stdout
pub fn write_str(s: &str) {
    interrupt::free(|_| unsafe {
        if let Some(stdout) = STDOUT.as_mut() {
            let _ = stdout.write_str(s);
        }
    })
}

/// Writes formatted string to stdout
pub fn write_fmt(args: fmt::Arguments) {
    interrupt::free(|_| unsafe {
        if let Some(stdout) = STDOUT.as_mut() {
            let _ = stdout.write_fmt(args);
        }
    })
}

/// Macro for printing to the serial standard output
#[macro_export]
macro_rules! sprint {
    ($s:expr) => {
        $crate::stdout::write_str($s)
    };
    ($($tt:tt)*) => {
        $crate::stdout::write_fmt(format_args!($($tt)*))
    };
}

/// Macro for printing to the serial standard output, with a newline.
#[macro_export]
macro_rules! sprintln {
    () => {
        $crate::stdout::write_str("\n")
    };
    ($s:expr) => {
        $crate::stdout::write_str(concat!($s, "\n"))
    };
    ($s:expr, $($tt:tt)*) => {
        $crate::stdout::write_fmt(format_args!(concat!($s, "\n"), $($tt)*))
    };
}
