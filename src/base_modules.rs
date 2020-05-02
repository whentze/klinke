const TAU: f32 = std::f32::consts::PI * 2.0;

use crate::{Module, cables::{Input, Output}, config::SAMPLE_RATE};

#[derive(Debug, Default)]
pub struct Sine {
    a: f32,
    b: f32,
    cos_b: f32,
    sin_a: f32,
    sin_a_old: f32,
}

impl Sine {
    fn set_freq(&mut self, hz: f32) {
        self.b = hz * TAU / SAMPLE_RATE;
        self.cos_b = self.b.cos();
        self.sin_a_old = (self.a-self.b).sin();
    }
}

impl Module for Sine {
    fn num_inputs(&self) -> u8 { 1 }
    fn num_outputs(&self) -> u8 { 1 }

    fn run(&mut self, i: &[Input], o: &[Output]) {
        self.set_freq(440.0 * 2.0f32.powf(i[0].get()));
        let new_sin = 2.0 * self.cos_b * self.sin_a - self.sin_a_old;
        self.a += self.b;
        self.sin_a_old = self.sin_a;
        self.sin_a = new_sin;
        &o[0] << new_sin;
    }
}

#[derive(Debug)]
pub struct Mixer;

impl Module for Mixer {
    fn num_inputs(&self) -> u8 { 8 }
    fn num_outputs(&self) -> u8 { 1 }

    fn run(&mut self, is: &[Input], o: &[Output]) {
        let mut count = 0;
        let mut sum = 0.0;
        for i in is {
            if i.is_connected() {
                count += 1;
                sum += i.get();
            }
        }
        &o[0] << if count > 0 { sum / count as f32 } else { 0.0 };
    }
}

#[derive(Debug, Default)]
pub struct Vca {}

impl Module for Vca {
    fn num_inputs(&self) -> u8 { 2 }
    fn num_outputs(&self) -> u8 { 1 }

    fn run(&mut self, i: &[Input], o: &[Output]) {
        &o[0] << i[0].get() * i[1].get();
    }
}

#[derive(Debug, Default)]
pub struct Vco {
    phase: f32,
}

impl Module for Vco {
    fn num_inputs(&self) -> u8 { 1 }
    fn num_outputs(&self) -> u8 { 1 }

    fn run(&mut self, i: &[Input], o: &[Output]) {
        let freq = 440.0 * 2.0f32.powf(i[0].get());

        &o[0] << 2.0*(self.phase % 1.0) - 1.0;

        self.phase += freq/SAMPLE_RATE;
    }
}