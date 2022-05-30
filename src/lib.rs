#![no_std]

use bsp::hal::adc::Adc;
use bsp::hal::ehal::digital::v2::OutputPin;
use bsp::hal::gpio::FloatingDisabled;
use bsp::hal::gpio::Pin;
use bsp::hal::gpio::B;
use bsp::hal::prelude::*;
use bsp::pac::ADC0;
use bsp::TouchXlId;
use bsp::TouchXlReset;
use bsp::TouchXrId;
use bsp::TouchXrReset;
use bsp::TouchYdId;
use bsp::TouchYdReset;
use bsp::TouchYuId;
use bsp::TouchYuReset;
use core::cmp::max;
use core::cmp::min;
use pyportal as bsp;

fn map_range(x: u16, in_min: u16, in_max: u16, out_min: u16, out_max: u16) -> u16 {
    let mapped = (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;

    if out_min <= out_max {
        max(min(mapped, out_max), out_min)
    } else {
        min(max(mapped, out_max), out_min)
    }
}

pub struct ResistiveTouchScreen {
    x_m: Pin<TouchXlId, FloatingDisabled>,
    x_p: Pin<TouchXrId, FloatingDisabled>,
    y_m: Pin<TouchYuId, FloatingDisabled>,
    y_p: Pin<TouchYdId, FloatingDisabled>,
    samples: u8,
    z_threshold: u16,
    calibration: ((u16, u16), (u16, u16)),
    size: (u16, u16),
}

impl ResistiveTouchScreen {
    pub fn new(
        x_m: impl Into<Pin<TouchXlId, FloatingDisabled>>,
        x_p: impl Into<Pin<TouchXrId, FloatingDisabled>>,
        y_m: impl Into<Pin<TouchYuId, FloatingDisabled>>,
        y_p: impl Into<Pin<TouchYdId, FloatingDisabled>>,
    ) -> Self {
        ResistiveTouchScreen {
            x_m: x_m.into(),
            x_p: x_p.into(),
            y_m: y_m.into(),
            y_p: y_p.into(),
            samples: 4,
            z_threshold: 10000,
            calibration: ((u16::MIN, u16::MAX), (u16::MIN, u16::MAX)),
            size: (u16::MAX, u16::MAX),
        }
    }

    pub fn release(self) -> (TouchXlReset, TouchXrReset, TouchYuReset, TouchYdReset) {
        (self.x_m, self.x_p, self.y_m, self.y_p)
    }

    pub fn read_touch(mut self, adc: &mut Adc<ADC0>) -> Result<Option<(u16, u16)>, ()> {
        let z = {
            let mut x_p = self.x_p.into_push_pull_output();
            let mut y_m = self.y_m.into_push_pull_output();
            let mut x_m = self.x_m.into_alternate::<B>();
            let mut y_p = self.y_p.into_alternate::<B>();

            x_p.set_low().map_err(|_| ())?;
            y_m.set_high().map_err(|_| ())?;
            let z1: u16 = adc.read(&mut x_m).map_err(|_| ())?;
            let z2: u16 = adc.read(&mut y_p).map_err(|_| ())?;

            let z = u16::MAX - (z2 - z1);

            self.x_p = x_p.into();
            self.y_m = y_m.into();
            self.x_m = x_m.into();
            self.y_p = y_p.into();

            z
        };
        if z > self.z_threshold {
            let x = {
                let mut x_p = self.x_p.into_push_pull_output();
                let mut x_m = self.x_m.into_push_pull_output();
                let mut y_p = self.y_p.into_alternate::<B>();
                x_p.set_high().map_err(|_| ())?;
                x_m.set_low().map_err(|_| ())?;

                let value: u16 = core::iter::from_fn(|| {
                    let sample: u16 = adc.read(&mut y_p).unwrap();
                    Some(sample)
                })
                .take(self.samples as usize)
                .sum::<u16>()
                    / (self.samples as u16);

                self.y_p = y_p.into();
                self.x_m = x_m.into();
                self.x_p = x_p.into();

                map_range(
                    value,
                    self.calibration.0 .0,
                    self.calibration.0 .1,
                    0,
                    self.size.0,
                )
            };

            let y = {
                let mut y_p = self.y_p.into_push_pull_output();
                let mut y_m = self.y_m.into_push_pull_output();
                let mut x_p = self.x_p.into_alternate::<B>();
                y_p.set_high().map_err(|_| ())?;
                y_m.set_low().map_err(|_| ())?;

                let value: u16 = core::iter::from_fn(|| {
                    let sample: u16 = adc.read(&mut x_p).unwrap();
                    Some(sample)
                })
                .take(self.samples as usize)
                .sum::<u16>()
                    / (self.samples as u16);

                self.y_p = y_p.into();
                self.y_m = y_m.into();
                self.x_p = x_p.into();

                map_range(
                    value,
                    self.calibration.1 .0,
                    self.calibration.1 .1,
                    0,
                    self.size.1,
                )
            };

            Ok(Some((x, y)))
        } else {
            Ok(None)
        }
    }
}
