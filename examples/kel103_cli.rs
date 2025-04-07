use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

use kel103::Kel103;

#[derive(Parser)]
struct Args {
    device: PathBuf,
    #[arg(default_value_t = 9600)]
    baud_rate: u32,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    DeviceInfo,
    GetVoltage,
    GetSetVoltage,
    SetVoltage {
        voltage: f32,
    },
    GetPower,
    GetSetPower,
    SetPower {
        watts: f32,
    },
    GetCurrent,
    GetSetCurrent,
    SetCurrent {
        amps: f32,
    },
    GetEnabled,
    SetEnabled {
        #[arg(action = ArgAction::Set)]
        enabled: bool,
    },
    SetConstantCurrent,
    SetConstantPower,
    SetConstantResistance,
    SetDynamicModeConstantVoltage {
        voltage1: f32,
        voltage2: f32,
        frequency: f32,
        duty_cycle: f32,
    },
    SetDynamicModeConstantCurrent {
        slope1: f32,
        slope2: f32,
        current1: f32,
        current2: f32,
        freq: f32,
        dutycycle: f32,
    },
    GetDynamicMode,
}
// --- Example Usage ---
fn main() {
    let args = Args::parse();

    let mut load = Kel103::new(args.device.as_path().to_str().unwrap(), args.baud_rate).unwrap();

    match args.command {
        Commands::DeviceInfo => println!("{}", load.device_info().unwrap()),
        Commands::GetVoltage => println!("{}", load.measure_volt().unwrap()),
        Commands::GetSetVoltage => println!("{}", load.measure_set_volt().unwrap()),
        Commands::SetVoltage { voltage } => load.set_volt(voltage).unwrap(),
        Commands::GetPower => println!("{}", load.measure_power().unwrap()),
        Commands::GetSetPower => println!("{}", load.measure_set_power().unwrap()),
        Commands::SetPower { watts } => load.set_power(watts).unwrap(),
        Commands::GetCurrent => println!("{}", load.measure_current().unwrap()),
        Commands::GetSetCurrent => println!("{}", load.measure_set_current().unwrap()),
        Commands::SetCurrent { amps } => load.set_current(amps).unwrap(),
        Commands::GetEnabled => println!("{}", load.check_output().unwrap()),
        Commands::SetEnabled { enabled } => load.set_output(enabled).unwrap(),
        Commands::SetConstantCurrent => load.set_constant_current().unwrap(),
        Commands::SetConstantPower => load.set_constant_power().unwrap(),
        Commands::SetConstantResistance => load.set_constant_resistance().unwrap(),
        Commands::SetDynamicModeConstantVoltage {
            voltage1,
            voltage2,
            frequency,
            duty_cycle,
        } => load
            .set_dynamic_mode_cv(voltage1, voltage2, frequency, duty_cycle)
            .unwrap(),
        Commands::SetDynamicModeConstantCurrent {
            slope1,
            slope2,
            current1,
            current2,
            freq,
            dutycycle,
        } => load
            .set_dynamic_mode_cc(slope1, slope2, current1, current2, freq, dutycycle)
            .unwrap(),
        Commands::GetDynamicMode => println!("{}", load.get_dynamic_mode().unwrap()),
    }
}
