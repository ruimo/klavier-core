use std::collections::BTreeMap;
use klavier_helper::store::{self, Store};

use crate::{channel::Channel, duration::Duration, pitch::Pitch, repeat::{AccumTick, Chunk}, tempo::TempoValue, velocity::Velocity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MidiSrc {
    NoteOn {
        channel: Channel,
        pitch: Pitch,
        velocity: Velocity,
    },
    NoteOff {
        channel: Channel,
        pitch: Pitch,
    },
    CtrlChg {
        channel: Channel,
        number: u8,
        velocity: Velocity,
    },
}

impl MidiSrc {
    fn render_to(self, buf: &mut Vec<u8>) {
        match self {
            MidiSrc::NoteOn {
                channel,
                pitch,
                velocity,
            } => {
                buf.push(0b10010000 | channel.as_u8());
                buf.push(pitch.value() as u8);
                buf.push(velocity.as_u8());
            }
            MidiSrc::NoteOff { channel, pitch } => {
                buf.push(0b10010000 | channel.as_u8());
                buf.push(pitch.value() as u8);
                buf.push(0);
            }
            MidiSrc::CtrlChg {
                channel,
                number,
                velocity,
            } => {
                buf.push(0b10110000 | channel.as_u8());
                buf.push(number);
                buf.push(velocity.as_u8());
            }
        }
    }
}


#[derive(Clone)]
pub struct PlayData {
    midi_data: Store<u64, Vec<Vec<u8>>, ()>,
    // Key: cycle, Value: tick
    table_for_tracking: Store<u64, (AccumTick, TempoValue), ()>,
    chunks: Store<AccumTick, Chunk, ()>,
}

impl PlayData {
    pub fn cycle_to_tick(&self, cycle: u64, sampling_rate: u32) -> AccumTick {
        let mut finder = self.table_for_tracking.finder();
        match finder.just_before(cycle) {
            Some((c, (tick, tempo))) => {
                tick + ((cycle - c) * tempo.as_u16() as u64 * Duration::TICK_RESOLUTION as u64
                    / sampling_rate as u64
                    / 60) as u32
            }
            None => {
                (cycle * TempoValue::default().as_u16() as u64 * Duration::TICK_RESOLUTION as u64
                    / sampling_rate as u64
                    / 60) as u32
            }
        }
    }

    pub fn accum_tick_to_tick(&self, tick: AccumTick) -> u32 {
        match self.chunks.finder().just_before(tick) {
            Some((at_tick, chunk)) => chunk.start_tick() + (tick - at_tick),
            None => tick,
        }
    }
}

#[derive(Clone)]
struct MidiEvents {
    events: BTreeMap<AccumTick, Vec<MidiSrc>>,
    tempo_table: Store<AccumTick, TempoValue, ()>,
    chunks: Store<AccumTick, Chunk, ()>,
}

impl MidiEvents {
    fn new(chunks: &[Chunk]) -> Self {
        Self {
            events: BTreeMap::new(),
            tempo_table: Store::new(false),
            chunks: Chunk::by_accum_tick(chunks),
        }
    }

    fn add_midi_event(&mut self, tick: AccumTick, m: MidiSrc) {
        match self.events.get_mut(&tick) {
            Some(found) => {
                found.push(m);
            }
            None => {
                self.events.insert(tick, vec![m]);
            }
        };
    }

    fn add_tempo(&mut self, tick: AccumTick, tempo: TempoValue) {
        self.tempo_table.add(tick, tempo, ());
    }

    fn cycles_by_accum_tick(
        &self,
        sampling_rate: usize,
        ticks_per_quarter: u32,
    ) -> Store<AccumTick, (TempoValue, u64), ()> {
        let mut cycles: u64 = 0;
        let mut buf = Store::with_capacity(self.tempo_table.len(), false);
        let mut prev_tick: AccumTick = 0;
        let mut prev_tempo: u16 = TempoValue::default().as_u16();

        for (tick, tempo) in self.tempo_table.iter() {
            cycles += (tick - prev_tick) as u64 * sampling_rate as u64 * 60
                / prev_tempo as u64
                / ticks_per_quarter as u64;
            buf.add(*tick, (*tempo, cycles), ());
            prev_tick = *tick;
            prev_tempo = tempo.as_u16();
        }

        buf
    }

    fn accum_tick_to_cycle(finder: &mut store::Finder<'_, u32, (TempoValue, u64), ()>, tick: AccumTick, sampling_rate: usize, ticks_per_quarter: u32) -> u64 {
        match finder.just_before(tick) {
            Some((t, (tempo, cycles))) => 
                *cycles + Self::tick_to_cycle(tick - *t, sampling_rate, tempo.as_u16(), ticks_per_quarter),
            None => 
                Self::tick_to_cycle(tick, sampling_rate, TempoValue::default().as_u16(), ticks_per_quarter),
        }
    }

    fn tick_to_cycle(tick: u32, sampling_rate: usize, tempo: u16, ticks_per_quarter: u32) -> u64 {
        tick as u64 * sampling_rate as u64 * 60
            / tempo as u64
            / ticks_per_quarter as u64
    }

    fn to_play_data(self, cycles_by_tick: Store<AccumTick, (TempoValue, u64), ()>, sampling_rate: usize, ticks_per_quarter: u32) -> PlayData {
        let mut cycles_by_tick = cycles_by_tick.finder();
        let mut midi_data = Store::new(false);
        let mut table_for_tracking = Store::new(false);

        for (tick, events) in self.events.iter() {
            let c = Self::accum_tick_to_cycle(&mut cycles_by_tick, *tick, sampling_rate, ticks_per_quarter);
            for e in events.iter() {
                let mut midi = vec![];
                e.render_to(&mut midi);

                midi_data.replace_mut(&c, (), |found: Option<&mut Vec<Vec<u8>>>| match found {
                    Some(current_midi_data) => {
                        current_midi_data.push(midi);
                        None
                    }
                    None => Some(vec![midi]),
                });
            }
        }

        for (tick, tempo) in self.tempo_table.iter() {
            let c = Self::accum_tick_to_cycle(&mut cycles_by_tick, *tick, sampling_rate, ticks_per_quarter);
            table_for_tracking.add(c, (*tick, tempo.clone()), ());
        }

        PlayData {
            midi_data,
            table_for_tracking,
            chunks: self.chunks,
        }
    }

//    fn play_start_cycle(&self, play_start_tick: PlayStartTick) -> u64 {
//
//    }
}

