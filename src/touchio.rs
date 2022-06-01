use atsamd_hal as hal;

use hal::adc::Adc;
use hal::adc::AdcChannel;
use hal::adc::AdcPeripheral;
use hal::adc::Reference;
use hal::adc::Resolution;
use hal::gpio::Pin;
use hal::gpio::AlternateB;
use hal::gpio::PushPullOutput;
use hal::gpio::PinId;
use hal::gpio::FloatingDisabled;
use hal::prelude::*;

pub type OutputPin<I> = Pin<I, PushPullOutput>;
pub type AdcPin<I> = Pin<I, AlternateB>;
pub type DisabledPin<I> = Pin<I, FloatingDisabled>;

pub enum TouchIO<I: PinId> {
    Empty,
    Output(OutputPin<I>),
    Adc(AdcPin<I>),
    Disabled(DisabledPin<I>),
}

impl<I: PinId> core::default::Default for TouchIO<I> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<I: PinId> TouchIO<I> {
    #[inline]
    fn unwrap_output(self) -> OutputPin<I> {
        match self {
            Self::Output(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    fn insert_output(&mut self, pin: OutputPin<I>) -> &mut OutputPin<I> {
        let _ = core::mem::replace(self, Self::Output(pin));
        match self {
            Self::Output(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    fn unwrap_adc(self) -> AdcPin<I> {
        match self {
            Self::Adc(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    fn insert_adc(&mut self, pin: AdcPin<I>) -> &mut AdcPin<I> {
        let _ = core::mem::replace(self, Self::Adc(pin));
        match self {
            Self::Adc(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    fn unwrap_disabled(self) -> DisabledPin<I> {
        match self {
            Self::Disabled(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    fn insert_disabled(&mut self, pin: DisabledPin<I>) -> &mut DisabledPin<I> {
        let _ = core::mem::replace(self, Self::Disabled(pin));
        match self {
            Self::Disabled(pin) => pin,
            _ => core::unreachable!(),
        }
    }

    #[inline]
    pub fn make_output(&mut self) -> &mut OutputPin<I> {
        match self {
            Self::Empty => panic!("No pin"),
            Self::Output(pin) => pin,
            Self::Adc(_) => {
                let pin = core::mem::take(self).unwrap_adc();
                self.insert_output(pin.into_mode())
            }
            Self::Disabled(_) => {
                let pin = core::mem::take(self).unwrap_disabled();
                self.insert_output(pin.into_mode())
            }
        }
    }

    #[inline]
    pub fn make_adc(&mut self) -> &mut AdcPin<I> {
        match self {
            Self::Empty => panic!("No pin"),
            Self::Adc(pin) => pin,
            Self::Output(_) => {
                let pin = core::mem::take(self).unwrap_output();
                self.insert_adc(pin.into_mode())
            }
            Self::Disabled(_) => {
                let pin = core::mem::take(self).unwrap_disabled();
                self.insert_adc(pin.into_mode())
            }
        }
    }

    pub fn make_disabled(&mut self) -> &mut DisabledPin<I> {
        match self {
            Self::Empty => panic!("No pin"),
            Self::Disabled(pin) => pin,
            Self::Adc(_) => {
                let pin = core::mem::take(self).unwrap_adc();
                self.insert_disabled(pin.into_mode())
            }
            Self::Output(_) => {
                let pin = core::mem::take(self).unwrap_output();
                self.insert_disabled(pin.into_mode())
            }
        }
    }

    #[inline]
    pub fn set_low(&mut self) {
        use embedded_hal::digital::v2::OutputPin;
        let pin = self.make_output();
        pin.set_low().ok();
    }

    #[inline]
    pub fn set_high(&mut self) {
        use embedded_hal::digital::v2::OutputPin;
        let pin = self.make_output();
        pin.set_high().ok();
    }

    #[inline]
    pub fn read<A>(&mut self, adc: &mut Adc<A>) -> u16
    where
        A: AdcPeripheral,
        I: AdcChannel<A>,
    {
        let pin = self.make_adc();

        adc.reference(Reference::INTVCC1);

        // SAMD21 only
        // adc.gain(Gain::DIV2);

        adc.resolution(Resolution::_12BIT);

        let _: u16 = adc.read(pin).unwrap(); // read twice, discard first result as recommended by data sheet
        let value: u16 = adc.read(pin).unwrap();

        (value << 4) | (value >> 8)
        // value
    }
}