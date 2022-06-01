#![no_std]

mod touchio;

use core::cmp::max;
use core::cmp::min;

use atsamd_hal as hal;

use hal::adc::Adc;
use hal::adc::AdcChannel;
use hal::adc::AdcPeripheral;
use hal::gpio::FloatingDisabled;
use hal::gpio::Pin;
use hal::gpio::PinId;
use touchio::TouchIO;

fn map_range(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
    let mapped: f32 = (x as f32 - in_min as f32) * (out_max as f32 - out_min as f32)
        / (in_max as f32 - in_min as f32)
        + out_min as f32;
    // let mapped = (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;

    if out_min <= out_max {
        mapped.min(out_max as f32).max(out_min as f32) as u32
        // max(min(mapped, out_max), out_min) as u16
    } else {
        mapped.max(out_max as f32).min(out_min as f32) as u32
        // min(max(mapped, out_max), out_min) as u16
    }
}

pub struct ResistiveTouchScreen<PinXM: PinId, PinXP: PinId, PinYM: PinId, PinYP: PinId> {
    x_m: TouchIO<PinXM>,
    x_p: TouchIO<PinXP>,
    y_m: TouchIO<PinYM>,
    y_p: TouchIO<PinYP>,
    samples: u8,
    z_threshold: u32,
    calibration: ((u32, u32), (u32, u32)),
    size: (u32, u32),
}

impl<PinXM: PinId, PinXP: PinId, PinYM: PinId, PinYP: PinId>
    ResistiveTouchScreen<PinXM, PinXP, PinYM, PinYP>
{
    pub fn new(
        x_m: impl Into<Pin<PinXM, FloatingDisabled>>,
        x_p: impl Into<Pin<PinXP, FloatingDisabled>>,
        y_m: impl Into<Pin<PinYM, FloatingDisabled>>,
        y_p: impl Into<Pin<PinYP, FloatingDisabled>>,
    ) -> Self {
        ResistiveTouchScreen {
            x_m: TouchIO::Disabled(x_m.into()),
            x_p: TouchIO::Disabled(x_p.into()),
            y_m: TouchIO::Disabled(y_m.into()),
            y_p: TouchIO::Disabled(y_p.into()),
            samples: 4,
            z_threshold: 10000,
            // calibration: ((0, 65535), (0, 65535)),
            calibration: ((13800, 52000), (16000, 44000)),
            // calibration: ((5200, 59000), (5800, 57000)),
            // size: (65535, 65535),
            size: (320, 240),
        }
    }

    // pub fn release(
    //     self,
    // ) -> (
    //     Pin<PinXM, FloatingDisabled>,
    //     Pin<PinXP, FloatingDisabled>,
    //     Pin<PinYM, FloatingDisabled>,
    //     Pin<PinYP, FloatingDisabled>,
    // ) {
    //     (self.x_m, self.x_p, self.y_m, self.y_p)
    // }

    pub fn touch_point<A: AdcPeripheral>(
        &mut self,
        adc: &mut Adc<A>,
    ) -> Option<(u32, u32, u32)>
    where
        PinXM: AdcChannel<A>,
        PinXP: AdcChannel<A>,
        PinYM: AdcChannel<A>,
        PinYP: AdcChannel<A>,
    {
        let z = {
            self.x_p.set_low();
            self.y_m.set_high();
            let z1 = self.x_m.read(adc);
            self.x_m.make_disabled();

            let z2 = self.y_p.read(adc);
            self.y_p.make_disabled();

            let z = 65535 - (z2 as u32 - z1 as u32);

            self.x_p.make_disabled();
            self.y_m.make_disabled();

            z
        };
        if z > self.z_threshold as u32 {
            let y = {
                self.x_p.set_high();
                self.x_m.set_low();

                let value = (0..self.samples)
                    .into_iter()
                    .map(|_| self.y_p.read(adc) as u32)
                    .sum::<u32>()
                    / (self.samples as u32);

                self.y_p.make_disabled();
                self.x_m.make_disabled();
                self.x_p.make_disabled();

                map_range(
                    value,
                    self.calibration.1 .0,
                    self.calibration.1 .1,
                    0,
                    self.size.1,
                )
            };

            let x = {
                self.y_p.set_high();
                self.y_m.set_low();

                let value = (0..self.samples)
                    .into_iter()
                    .map(|_| self.x_p.read(adc) as u32)
                    .sum::<u32>()
                    / (self.samples as u32);

                self.x_p.make_disabled();
                self.y_p.make_disabled();
                self.y_m.make_disabled();

                map_range(
                    value,
                    self.calibration.0 .0,
                    self.calibration.0 .1,
                    0,
                    self.size.0,
                )
            };

            Some((x, y, z))
        } else {
            None
        }
    }
}
