#![warn(missing_docs)]
//! A crate for controlling KEL103 Electronic Loads
//! Currently only serial port control is supported, but adding UDP control
//! should be simple.

use serialport::{SerialPort, TTYPort};
use std::{
    io::{self, BufRead, BufReader},
    time::Duration,
};
use thiserror::Error;

// Define custom errors for better context
#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum KelError {
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to parse float value: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Received invalid UTF-8 data from device: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Value set incorrectly on the device: {0}")]
    ValueError(String),
    #[error("Device communication error: {0}")]
    DeviceError(String),
    #[error("Device is not a KEL103")]
    DeviceModel(String),
}

type Result<T> = std::result::Result<T, KelError>;

/// Representation of a KEL103 Electronic Load
pub struct Kel103 {
    port_write: Box<dyn SerialPort>,
    port_read: BufReader<TTYPort>,
}

impl Kel103 {
    /// Attempt to create a KEL103 from a serial port and baud rate
    /// On Linux serial port should be a path (e.g `/dev/ttyACM0`),
    /// on windows it will be a port name (e.g `COM0`).
    pub fn new(serial_port: &str, baud_rate: u32) -> Result<Self> {
        let port = serialport::new(serial_port, baud_rate)
            .timeout(Duration::from_secs(1))
            .open_native()?;
        let (port_write, port_read) = (port.try_clone()?, BufReader::new(port));

        let mut this = Kel103 {
            port_write,
            port_read,
        };
        let info = this.device_info()?;
        if !info.contains("KEL103") {
            return Err(KelError::DeviceModel(info));
        };

        Ok(this)
    }

    /// Get device identification string.
    pub fn device_info(&mut self) -> Result<String> {
        self.send_recv(b"*IDN?")
    }

    /// Measure the input voltage.
    pub fn measure_volt(&mut self) -> Result<f32> {
        let s = self.send_recv(b":MEAS:VOLT?")?;
        let val_str = s.trim_end_matches(['V', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from) // Convert parse error
    }

    /// Measure the *set* (CV mode) voltage level.
    pub fn measure_set_volt(&mut self) -> Result<f32> {
        let s = self.send_recv(b":VOLT?")?;
        let val_str = s.trim_end_matches(['V', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from)
    }

    /// Set the voltage level (CV mode).
    pub fn set_volt(&mut self, voltage: f32) -> Result<()> {
        let cmd = format!(":VOLT {:.3}V", voltage); // Format voltage
        self.send(cmd.as_bytes())?;
        // Verification - Note: direct float comparison can be problematic
        let set_v = self.measure_set_volt()?;
        if (set_v - voltage).abs() > 1e-9 {
            // Using a small tolerance instead of !=
            return Err(KelError::ValueError(format!(
                "Voltage set incorrectly on the device. Expected {}, got {}",
                voltage, set_v
            )));
        }
        Ok(())
    }

    /// Measure the input power.
    pub fn measure_power(&mut self) -> Result<f32> {
        let s = self.send_recv(b":MEAS:POW?")?;
        let val_str = s.trim_end_matches(['W', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from)
    }

    /// Measure the *set* power level.
    pub fn measure_set_power(&mut self) -> Result<f32> {
        let s = self.send_recv(b":POW?")?;
        let val_str = s.trim_end_matches(['W', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from)
    }

    /// Set the power level (CW mode).
    pub fn set_power(&mut self, power: f32) -> Result<()> {
        let cmd = format!(":POW {:.3}W", power);
        self.send(cmd.as_bytes())?;
        // Verification
        let set_p = self.measure_set_power()?;
        if (set_p - power).abs() > 1e-9 {
            // Use tolerance
            return Err(KelError::ValueError(format!(
                "Power set incorrectly on the device. Expected {}, got {}",
                power, set_p
            )));
        }
        Ok(())
    }

    /// Measure the input current.
    pub fn measure_current(&mut self) -> Result<f32> {
        let s = self.send_recv(b":MEAS:CURR?")?;
        let val_str = s.trim_end_matches(['A', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from)
    }

    /// Measure the *set* current level.
    pub fn measure_set_current(&mut self) -> Result<f32> {
        let s = self.send_recv(b":CURR?")?;
        let val_str = s.trim_end_matches(['A', '\n', '\r'].as_ref()).trim();
        val_str.parse::<f32>().map_err(KelError::from)
    }

    /// Set the current level (CC mode).
    pub fn set_current(&mut self, current: f32) -> Result<()> {
        let cmd = format!(":CURR {:.3}A", current);
        self.send(cmd.as_bytes())?;
        // Verification
        let set_c = self.measure_set_current()?;
        if (set_c - current).abs() > 1e-9 {
            // Use tolerance
            return Err(KelError::ValueError(format!(
                "Current set incorrectly on the device. Expected {}, got {}",
                current, set_c
            )));
        }
        Ok(())
    }

    /// Check if the input/output is enabled (ON) or disabled (OFF).
    pub fn check_output(&mut self) -> Result<bool> {
        let s = self.send_recv(b":INP?")?;
        if s.contains("OFF") {
            Ok(false)
        } else if s.contains("ON") {
            Ok(true)
        } else {
            Err(KelError::DeviceError(format!(
                "Unexpected response from :INP?: {}",
                s
            )))
        }
    }

    /// Enable (true) or disable (false) the input/output.
    pub fn set_output(&mut self, state: bool) -> Result<()> {
        let cmd = if state { b":INP 1" } else { b":INP 0" };
        self.send(cmd)?;
        // Verification
        let actual_state = self.check_output()?;
        if actual_state != state {
            return Err(KelError::ValueError(format!(
                "Caution: Output not set correctly. Expected {}, got {}",
                state, actual_state
            )));
        }
        Ok(())
    }

    /// Set the device mode to Constant Current (CC).
    pub fn set_constant_current(&mut self) -> Result<()> {
        self.send(b":FUNC CC")
    }

    /// Set the device mode to Constant Power (CW).
    pub fn set_constant_power(&mut self) -> Result<()> {
        self.send(b":FUNC CW")
    }

    /// Set the device mode to Constant Resistance (CR).
    pub fn set_constant_resistance(&mut self) -> Result<()> {
        self.send(b":FUNC CR")
    }

    /// Set Dynamic Mode CV (Constant Voltage).
    pub fn set_dynamic_mode_cv(
        &mut self,
        voltage1: f32,
        voltage2: f32,
        freq: f32,
        dutycycle: f32,
    ) -> Result<()> {
        let cmd = format!(
            ":DYN 1,{:.3}V,{:.3}V,{:.3}HZ,{:.3}%",
            voltage1, voltage2, freq, dutycycle
        );
        self.send(cmd.as_bytes())
    }

    /// Set Dynamic Mode CC (Constant Current).
    pub fn set_dynamic_mode_cc(
        &mut self,
        slope1: f32,
        slope2: f32,
        current1: f32,
        current2: f32,
        freq: f32,
        dutycycle: f32,
    ) -> Result<()> {
        let cmd = format!(
            ":DYN 2,{:.3}A/uS,{:.3}A/uS,{:.3}A,{:.3}A,{:.3}HZ,{:.3}%",
            slope1, slope2, current1, current2, freq, dutycycle
        );
        self.send(cmd.as_bytes())
    }

    /// Get the current dynamic mode settings.
    pub fn get_dynamic_mode(&mut self) -> Result<String> {
        let s = self.send_recv(b":DYN?")?;
        Ok(s.trim_end_matches('\n').to_string())
    }

    /// Sends a message and receives a response line.
    fn send_recv(&mut self, message: &[u8]) -> Result<String> {
        // Write message with newline
        self.send(message)?;

        // Read response line
        let mut response_bytes = Vec::new();
        self.port_read.read_until(b'\n', &mut response_bytes)?; // Read until newline

        // Convert to UTF-8 String
        let response_str = String::from_utf8(response_bytes)?; // Propagate UTF8 errors

        Ok(response_str) // Port closed automatically when `port` and `buf_reader` go out of scope
    }

    fn send(&mut self, message: &[u8]) -> Result<()> {
        // Write message with newline
        self.port_write.write_all(message)?; // Propagate IO errors
        self.port_write.write_all(b"\n")?;
        self.port_write.flush()?; // Ensure data is sent

        Ok(())
    }
}
