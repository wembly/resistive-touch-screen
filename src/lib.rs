#![no_std]

use bsp::hal::adc::Adc;
use bsp::hal::ehal::adc::Channel;
use bsp::hal::ehal::digital::v2::OutputPin;
use bsp::hal::gpio::Alternate;
use bsp::hal::gpio::AnyPin;
use bsp::hal::gpio::Pin;
use bsp::hal::gpio::PushPullOutput;
use bsp::hal::gpio::B;
use bsp::hal::prelude::*;
use bsp::pac::ADC0;
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

pub struct ResistiveTouchPanel<PinXM, PinXP, PinYM, PinYP> {
    x_m: PinXM,
    x_p: PinXP,
    y_m: PinYM,
    y_p: PinYP,
    samples: u8,
    z_threshold: u16,
    calibration: ((u16, u16), (u16, u16)),
    size: (u16, u16),
}

impl<PinXM, PinXP, PinYM, PinYP> ResistiveTouchPanel<PinXM, PinXP, PinYM, PinYP>
where
    PinXM: AnyPin + From<Pin<<PinXM as AnyPin>::Id, Alternate<B>>>,
    PinXP: AnyPin
        + From<Pin<<PinXP as AnyPin>::Id, PushPullOutput>>
        + From<Pin<<PinXP as AnyPin>::Id, Alternate<B>>>,
    PinYM: AnyPin + From<Pin<<PinYM as AnyPin>::Id, PushPullOutput>>,
    PinYP: AnyPin
        + From<Pin<<PinYP as AnyPin>::Id, PushPullOutput>>
        + From<Pin<<PinYP as AnyPin>::Id, Alternate<B>>>,
    Pin<<PinXM as AnyPin>::Id, Alternate<B>>: Channel<ADC0>,
    Pin<<PinYP as AnyPin>::Id, Alternate<B>>: Channel<ADC0>,
    Pin<<PinXP as AnyPin>::Id, Alternate<B>>: Channel<ADC0>,
{
    pub fn new(x_m: PinXM, x_p: PinXP, y_m: PinYM, y_p: PinYP) -> Self {
        ResistiveTouchPanel {
            x_m: x_m,
            x_p: x_p,
            y_m: y_m,
            y_p: y_p,
            samples: 4,
            z_threshold: 10000,
            calibration: ((u16::MIN, u16::MAX), (u16::MIN, u16::MAX)),
            size: (u16::MAX, u16::MAX),
        }
    }

    pub fn release(self) -> (PinXM, PinXP, PinYM, PinYP) {
        (self.x_m, self.x_p, self.y_m, self.y_p)
    }

    pub fn read_touch(mut self, adc: &mut Adc<ADC0>) -> Result<Option<(u16, u16)>, ()> {
        let z = {
            let mut x_p = self.x_p.into().into_push_pull_output();
            let mut y_m = self.y_m.into().into_push_pull_output();
            let mut x_m = self.x_m.into().into_alternate::<B>();
            let mut y_p = self.y_p.into().into_alternate::<B>();

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
                let mut x_p = self.y_p.into().into_push_pull_output();
                let mut x_m = self.y_m.into().into_push_pull_output();
                let mut y_p = self.x_p.into().into_alternate::<B>();
                x_p.set_high().map_err(|_| ())?;
                x_m.set_low().map_err(|_| ())?;

                let value: u16 = core::iter::from_fn(|| {
                    let sample: u16 = adc.read(&mut y_p).unwrap();
                    Some(sample)
                })
                .take(self.samples as usize)
                .sum::<u16>()
                    / (self.samples as u16);

                self.y_p = x_p.into();
                self.y_m = x_m.into();
                self.x_p = y_p.into();

                map_range(
                    value,
                    self.calibration.0 .0,
                    self.calibration.0 .1,
                    0,
                    self.size.0,
                )
            };

            let y = {
                let mut y_p = self.y_p.into().into_push_pull_output();
                let mut y_m = self.y_m.into().into_push_pull_output();
                let mut x_p = self.x_p.into().into_alternate::<B>();
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
                    self.size.0,
                )
            };

            Ok(Some((x, y)))
        } else {
            Ok(None)
        }
    }
}
