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
    pub fn new() -> Self {
        let mut s = Self::default();
        s.set_freq(440.0);
        s
    }
    fn set_freq(&mut self, hz: f32) {
        self.b = hz * TAU / SAMPLE_RATE;
        self.cos_b = self.b.cos();
        self.sin_a_old = (self.a-self.b).sin();
    }
}

impl Module for Sine {
    fn num_inputs(&self) -> u8 { 0 }
    fn num_outputs(&self) -> u8 { 1 }

    fn run(&mut self, _: &[Input], o: &[Output]) {
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