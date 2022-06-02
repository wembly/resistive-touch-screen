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

fn map_range(x: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
    let mapped: f32 = (x as f32 - in_min as f32) * (out_max as f32 - out_min as f32)
        / (in_max as f32 - in_min as f32)
        + out_min as f32;
    // let mapped = (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;

    if out_min <= out_max {
        mapped.min(out_max as f32).max(out_min as f32) as i32
        // max(min(mapped, out_max), out_min) as u16
    } else {
        mapped.max(out_max as f32).min(out_min as f32) as i32
        // min(max(mapped, out_max), out_min) as u16
    }
}

pub struct ResistiveTouchScreen<PinXM: PinId, PinXP: PinId, PinYM: PinId, PinYP: PinId> {
    x_m: TouchIO<PinXM>,
    x_p: TouchIO<PinXP>,
    y_m: TouchIO<PinYM>,
    y_p: TouchIO<PinYP>,
    samples: u8,
    z_threshold: u16,
    calibration: ((u16, u16), (u16, u16)),
    size: (usize, usize),
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
            calibration: ((u16::MIN, u16::MAX), (u16::MIN, u16::MAX)),
            size: (u16::MAX.into(), u16::MAX.into()),
        }
    }

    pub fn samples(mut self, samples: u8) -> Self {
        self.samples = samples;
        self
    }

    pub fn z_threshold(mut self, z_threshold: u16) -> Self {
        self.z_threshold = z_threshold;
        self
    }

    pub fn calibration(mut self, x_min: u16, x_max: u16, y_min: u16, y_max: u16) -> Self {
        self.calibration = ((x_min, x_max), (y_min, y_max));
        self
    }

    pub fn size(mut self, x: usize, y: usize) -> Self {
        self.size = (x, y);
        self
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

    pub fn touch_point<A: AdcPeripheral>(&mut self, adc: &mut Adc<A>) -> Option<(i32, i32, i32)>
    where
        PinXM: AdcChannel<A>,
        PinXP: AdcChannel<A>,
        PinYM: AdcChannel<A>,
        PinYP: AdcChannel<A>,
    {
        let z = {
            self.x_p.set_low();
            self.y_m.set_high();
            let z1: i32 = self.x_m.read(adc).into();
            self.x_m.make_disabled();

            let z2: i32 = self.y_p.read(adc).into();
            self.y_p.make_disabled();

            let z: i32 = (u16::MAX as i32) - (z2 - z1);

            self.x_p.make_disabled();
            self.y_m.make_disabled();

            z
        };
        if z > self.z_threshold as i32 {
            let y = {
                self.x_p.set_high();
                self.x_m.set_low();

                let value = (0..self.samples)
                    .into_iter()
                    .map(|_| self.y_p.read(adc) as i32)
                    .sum::<i32>()
                    / (self.samples as i32);

                self.y_p.make_disabled();
                self.x_m.make_disabled();
                self.x_p.make_disabled();

                map_range(
                    value as i32,
                    self.calibration.1 .0 as i32,
                    self.calibration.1 .1 as i32,
                    0_i32,
                    self.size.1 as i32,
                )
            };

            let x = {
                self.y_p.set_high();
                self.y_m.set_low();

                let value = (0..self.samples)
                    .into_iter()
                    .map(|_| self.x_p.read(adc) as i32)
                    .sum::<i32>()
                    / (self.samples as i32);

                self.x_p.make_disabled();
                self.y_p.make_disabled();
                self.y_m.make_disabled();

                map_range(
                    value as i32,
                    self.calibration.0 .0 as i32,
                    self.calibration.0 .1 as i32,
                    0_i32,
                    self.size.0 as i32,
                )
            };

            Some((x, y, z))
        } else {
            None
        }
    }
}
