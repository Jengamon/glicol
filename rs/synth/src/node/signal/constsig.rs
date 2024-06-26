use crate::{Buffer, Input, Message, Node};
use hashbrown::HashMap;
#[derive(Debug, Clone)]
pub struct ConstSig {
    val: f32,
    events: Vec<(f32, f32)>,
    pattern: Vec<(f32, f32)>,
    span: f32,
    bpm: f32,
    sr: usize,
    step: usize,
    input_order: Vec<usize>,
}

impl ConstSig {
    pub fn new(val: f32) -> Self {
        Self {
            val,
            events: vec![],
            pattern: vec![],
            span: 1.,
            bpm: 120.,
            sr: 44100,
            step: 0,
            input_order: Vec::new(),
        }
    }

    pub fn events(self, events: Vec<(f32, f32)>) -> Self {
        Self { events, ..self }
    }

    pub fn pattern(self, pattern: Vec<(f32, f32)>) -> Self {
        Self { pattern, ..self }
    }

    pub fn span(self, span: f32) -> Self {
        Self { span, ..self }
    }

    pub fn bpm(self, bpm: f32) -> Self {
        Self { bpm, ..self }
    }

    pub fn sr(self, sr: usize) -> Self {
        Self { sr, ..self }
    }
}

impl<const N: usize> Node<N> for ConstSig {
    fn process(&mut self, _inputs: &mut HashMap<usize, Input<N>>, output: &mut [Buffer<N>]) {
        let cycle_dur = 60. / self.bpm * 4.;
        let bar_dur = cycle_dur * self.span * self.sr as f32;

        for out in &mut *output[0] {
            for event in &self.events {
                if (self.step % (bar_dur as usize))
                    == ((event.1 * cycle_dur * self.sr as f32) as usize)
                {
                    self.val = event.0
                }
            }

            for event in &self.pattern {
                if (self.step % (bar_dur as usize))
                    == ((event.1 * cycle_dur * self.sr as f32) as usize)
                {
                    self.val = event.0
                }
            }

            *out = self.val;
            self.step += 1;
        }
    }
    fn send_msg(&mut self, info: Message) {
        match info {
            Message::SetPattern(p, span) => {
                self.pattern = p;
                self.span = span;
            }
            Message::SetToNumber(0, value) => self.val = value,
            Message::SetBPM(bpm) => self.bpm = bpm,
            Message::Index(i) => self.input_order.push(i),
            Message::IndexOrder(pos, index) => self.input_order.insert(pos, index),
            _ => {}
        }
    }
}
