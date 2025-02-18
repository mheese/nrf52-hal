//! HAL interface to the TIMER peripheral
//!
//! See product specification, chapter 24.

use core::ops::Deref;

#[cfg(feature="9160")]
use crate::target::{timer0_ns as timer0, Interrupt, NVIC, TIMER0_NS as TIMER0, TIMER1_NS as TIMER1, TIMER2_NS as TIMER2};

#[cfg(not(feature="9160"))]
use crate::target::{timer0, Interrupt, NVIC, TIMER0, TIMER1, TIMER2};

use embedded_hal::{prelude::*, timer};
use nb::{self, block};
use void::{unreachable, Void};

#[cfg(any(feature = "52832", feature = "52840"))]
use crate::target::{TIMER3, TIMER4};

use core::marker::PhantomData;

pub struct OneShot;
pub struct Periodic;


/// Interface to a TIMER instance
///
/// Right now, this is a very basic interface. The timer will always be
/// hardcoded to a frequency of 1 MHz and 32 bits accuracy.
///
/// CC[0] is used for the current/most-recent delay period and CC[1] is used
/// to grab the current value of the counter at a given instant.
pub struct Timer<T, U = OneShot>(T, PhantomData<U>);

impl<T> Timer<T, OneShot>
where 
    T: Instance,
{
    pub fn one_shot(timer: T) -> Timer<T, OneShot> {
        timer
            .shorts
            .write(|w| w.compare0_clear().enabled().compare0_stop().enabled());
        timer.prescaler.write(
            |w| unsafe { w.prescaler().bits(4) }, // 1 MHz
        );
        timer.bitmode.write(|w| w.bitmode()._32bit());

        Timer::<T, OneShot>(timer, PhantomData)
    }

    pub fn new(timer: T) -> Timer<T, OneShot> {
        Timer::<T, OneShot>::one_shot(timer)
    }

}

impl<T> Timer<T, Periodic>
where
    T: Instance,
{
    pub fn periodic(timer: T) -> Timer<T, Periodic> {
        timer
            .shorts
            .write(|w| w.compare0_clear().enabled().compare0_stop().disabled());
        timer.prescaler.write(
            |w| unsafe { w.prescaler().bits(4) }, // 1 MHz
        );
        timer.bitmode.write(|w| w.bitmode()._32bit());

        Timer::<T, Periodic>(timer, PhantomData)
    }

}

impl<T, U> Timer<T, U>
where
    T: Instance,
{
    pub const TICKS_PER_SECOND: u32 = 1_000_000;

    pub fn into_periodic(self) -> Timer<T, Periodic> {
        self.0
            .shorts
            .write(|w| w.compare0_clear().enabled().compare0_stop().disabled());

        Timer::<T, Periodic>(self.free(), PhantomData)
    }

    pub fn into_oneshot(self) -> Timer<T, OneShot> {
        self.0
            .shorts
            .write(|w| w.compare0_clear().enabled().compare0_stop().enabled());

        Timer::<T, OneShot>(self.free(), PhantomData)
    }

    /// Return the raw interface to the underlying timer peripheral
    pub fn free(self) -> T {
        self.0
    }

    /// Return the current value of the counter, by capturing to CC[1].
    pub fn read(&self) -> u32 {
        self.0.tasks_capture[1].write(|w| unsafe { w.bits(1) });
        self.0.cc[1].read().bits()
    }

    /// Enables the interrupt for this timer
    ///
    /// Enables an interrupt that is fired when the timer reaches the value that
    /// is given as an argument to `start`.
    ///
    /// If access to the NVIC is not provided, the interrupt must ALSO be enabled
    /// there outside of this function (e.g. manually call `nvic.enable`, or through
    /// the use of RTFM).
    pub fn enable_interrupt(&mut self, nvic: Option<&mut NVIC>) {
        // As of this writing, the timer code only uses
        // `cc[0]`/`events_compare[0]`. If the code is extended to use other
        // compare registers, the following needs to be adapted.
        self.0.intenset.modify(|_, w| w.compare0().set());

        if let Some(_nvic) = nvic {
            unsafe { NVIC::unmask(T::INTERRUPT) };
        }
    }

    /// Disables the interrupt for this timer
    ///
    /// Disables an interrupt that is fired when the timer reaches the value
    /// that is given as an argument to `start`.
    ///
    /// If access to the NVIC is not provided, the interrupt must ALSO be disabled
    /// there outside of this function (e.g. manually call `nvic.disable`, or through
    /// the use of RTFM).
    pub fn disable_interrupt(&mut self, nvic: Option<&mut NVIC>) {
        // As of this writing, the timer code only uses
        // `cc[0]`/`events_compare[0]`. If the code is extended to use other
        // compare registers, the following needs to be adapted.
        self.0.intenclr.modify(|_, w| w.compare0().clear());

        if let Some(_nvic) = nvic {
            NVIC::mask(T::INTERRUPT);
        }
    }

    pub fn delay(&mut self, cycles: u32) {
        self.start(cycles);
        match block!(self.wait()) {
            Ok(_) => {}
            Err(x) => unreachable(x),
        }
    }
}

impl<T, U> timer::CountDown for Timer<T, U>
where
    T: Instance,
{
    type Time = u32;

    /// Start the timer
    ///
    /// The timer will run for the given number of cycles, then it will stop and
    /// reset.
    fn start<Time>(&mut self, cycles: Time)
    where
        Time: Into<Self::Time>,
    {
        // If the following sequence of events occurs, the COMPARE event will be
        // set here:
        // 1. `start` is called.
        // 2. The timer runs out but `wait` is _not_ called.
        // 3. `start` is called again
        //
        // If that happens, then we need to reset the event here explicitly, as
        // nothing else this method does will reset the event, and if it's still
        // active after this method exits, then the next call to `wait` will
        // return immediately, no matter how much time has actually passed.
        self.0.events_compare[0].reset();

        // Configure timer to trigger EVENTS_COMPARE when given number of cycles
        // is reached.
        self.0.cc[0].write(|w|
            // The timer mode was set to 32 bits above, so all possible values
            // of `cycles` are valid.
            unsafe { w.cc().bits(cycles.into()) });

        // Clear the counter value
        self.0.tasks_clear.write(|w| unsafe { w.bits(1) });

        // Start the timer
        self.0.tasks_start.write(|w| unsafe { w.bits(1) });
    }

    /// Wait for the timer to stop
    ///
    /// Will return `Err(nb::Error::WouldBlock)` while the timer is still
    /// running. Once the timer reached the number of cycles given in the
    /// `start` method, it will return `Ok(())`.
    ///
    /// To block until the timer has stopped, use the `block!` macro from the
    /// `nb` crate. Please refer to the documentation of `nb` for other options.
    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.0.events_compare[0].read().bits() == 0 {
            // EVENTS_COMPARE has not been triggered yet
            return Err(nb::Error::WouldBlock);
        }

        // Reset the event, otherwise it will always read `1` from now on.
        self.0.events_compare[0].write(|w| w);

        Ok(())
    }
}

impl<T, U> timer::Cancel for Timer<T, U>
where
    T: Instance,
{
    type Error = ();

    // Cancel a running timer.
    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.0.tasks_stop.write(|w| unsafe { w.bits(1) });
        self.0.events_compare[0].write(|w| w);
        Ok(())
    }
}

impl<T> timer::Periodic for Timer<T, Periodic>
where
    T: Instance,
{}


/// Implemented by all `TIMER` instances
pub trait Instance: Deref<Target = timer0::RegisterBlock> {
    /// This interrupt associated with this RTC instance
    const INTERRUPT: Interrupt;
}

macro_rules! impl_instance {
    ($($name:ident,)*) => {
        $(
            impl Instance for $name {
                const INTERRUPT: Interrupt = Interrupt::$name;
            }
        )*
    }
}

impl_instance!(TIMER0, TIMER1, TIMER2,);

#[cfg(any(feature = "52832", feature = "52840"))]
impl_instance!(TIMER3, TIMER4,);
