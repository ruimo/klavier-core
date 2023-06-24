use std::{rc::Rc, io::Cursor};

use serde_json::Value;

use crate::{note::Note, bar::Bar, tempo::Tempo, ctrl_chg::CtrlChg};

#[derive(Clone, PartialEq, Debug, serde::Deserialize, serde::Serialize)]
pub struct Models {
    pub notes: Vec<Note>,
    pub bars: Vec<Bar>,
    pub tempos: Vec<Tempo>,
    pub dumpers: Vec<CtrlChg>,
    pub softs: Vec<CtrlChg>,
}

#[derive(Debug, PartialEq)]
pub enum FromClipboardTextErr {
    VersionErr { detected_ver: u64 },
    CannotParse { err_json: String, detail: String },
    EmptyString,
    VersionNotU64 { err_json: String },
}

impl Models {
    pub const VERSION: u64 = 1;

    #[inline]
    pub fn unwrap_rc(notes: &[Rc<Note>]) -> Vec<Note> {
        notes.iter().map(|n| (**n).clone()).collect()
    }

    #[inline]
    pub fn with_capacity(note: usize, bar: usize, tempo: usize, dumper: usize, soft: usize) -> Self {
        Self {
            notes: Vec::with_capacity(note),
            bars: Vec::with_capacity(bar),
            tempos: Vec::with_capacity(tempo),
            dumpers: Vec::with_capacity(dumper),
            softs: Vec::with_capacity(soft),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        }
    }

    pub fn move_to_tick(mut self, tick: u32) -> Self {
        let mut smallest_tick: u32 = u32::MAX;
        for n in self.notes.iter() {
            smallest_tick = std::cmp::min(n.base_start_tick, smallest_tick);
        }
        for b in self.bars.iter() {
            smallest_tick = std::cmp::min(b.start_tick, smallest_tick);
        }
        for t in self.tempos.iter() {
            smallest_tick = std::cmp::min(t.start_tick, smallest_tick);
        }
        for d in self.dumpers.iter() {
            smallest_tick = std::cmp::min(d.start_tick, smallest_tick);
        }
        for s in self.softs.iter() {
            smallest_tick = std::cmp::min(s.start_tick, smallest_tick);
        }

        let offset: i64 = (tick as i64) - (smallest_tick as i64);
        if offset == 0 { return self }

        for n in self.notes.iter_mut() {
            n.base_start_tick = ((n.base_start_tick as i64) + offset) as u32;
        }
        for b in self.bars.iter_mut() {
            b.start_tick = ((b.start_tick as i64) + offset) as u32;
        }
        for t in self.tempos.iter_mut() {
            t.start_tick = ((t.start_tick as i64) + offset) as u32;
        }
        for d in self.dumpers.iter_mut() {
            d.start_tick = ((d.start_tick as i64) + offset) as u32;
        }
        for s in self.softs.iter_mut() {
            s.start_tick = ((s.start_tick as i64) + offset) as u32;
        }

        self
    }

    pub fn with_notes(mut self, notes: &[Rc<Note>]) -> Self {
        self.notes = Self::unwrap_rc(notes);
        self
    }

    pub fn with_bars(mut self, bars: Vec<Bar>) -> Self {
        self.bars = bars;
        self
    }

    pub fn with_tempos(mut self, tempos: Vec<Tempo>) -> Self {
        self.tempos = tempos;
        self
    }

    pub fn with_dumpers(mut self, dumpers: Vec<CtrlChg>) -> Self {
        self.dumpers = dumpers;
        self
    }

    pub fn with_softs(mut self, softs: Vec<CtrlChg>) -> Self {
        self.softs = softs;
        self
    }

    pub fn to_clipboard_text(&self) -> String {
        use std::io::prelude::*;

        let mut c = Cursor::new(Vec::new());
        c.write_all(b"1").unwrap();
        serde_json::to_writer(&mut c, self).unwrap();
        String::from_utf8_lossy(c.get_ref()).into_owned()
    }

    pub fn from_clipboard_text(json: String) -> Result<Self, FromClipboardTextErr> {
        let mut stream = serde_json::Deserializer::from_str(&json).into_iter::<Value>();
        match stream.next() {
            None => return Err(FromClipboardTextErr::EmptyString),
            Some(Ok(ver)) =>
                if let Value::Number(ver_no) = ver {
                    if let Some(v) = ver_no.as_u64() {
                        if v == Self::VERSION {
                            serde_json::from_slice::<'_, Models>(&json.as_bytes()[stream.byte_offset()..])
                                .map_err(|e| FromClipboardTextErr::CannotParse { err_json: json, detail: e.to_string() })
                        } else {
                            Err(FromClipboardTextErr::VersionErr { detected_ver: v })
                        }
                    } else {
                        Err(FromClipboardTextErr::VersionNotU64 { err_json: json })
                    }
                } else {
                    Err(FromClipboardTextErr::VersionNotU64 { err_json: json })
                },
            Some(Err(e)) => Err(FromClipboardTextErr::CannotParse { err_json: json, detail: e.to_string() })
        }
    }
}

pub struct ModelChanges {
    pub notes: Vec<(Note, Note)>,
    pub bars: Vec<(Bar, Bar)>,
    pub tempos: Vec<(Tempo, Tempo)>,
    pub dumpers: Vec<(CtrlChg, CtrlChg)>,
    pub softs: Vec<(CtrlChg, CtrlChg)>,
}

impl ModelChanges {
    #[inline]
    pub fn empty() -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        }
    }

    #[inline]
    pub fn with_capacity(note: usize, bar: usize, tempo: usize, dumper: usize, soft: usize) -> Self {
        Self {
            notes: Vec::with_capacity(note),
            bars: Vec::with_capacity(bar),
            tempos: Vec::with_capacity(tempo),
            dumpers: Vec::with_capacity(dumper),
            softs: Vec::with_capacity(soft),
        }
    }

    pub fn with_notes(mut self, notes: Vec<(Note, Note)>) -> Self {
        self.notes = notes;
        self
    }

    pub fn with_bars(mut self, bars: Vec<(Bar, Bar)>) -> Self {
        self.bars = bars;
        self
    }

    pub fn with_tempos(mut self, tempos: Vec<(Tempo, Tempo)>) -> Self {
        self.tempos = tempos;
        self
    }

    pub fn with_dumpers(mut self, dumpers: Vec<(CtrlChg, CtrlChg)>) -> Self {
        self.dumpers = dumpers;
        self
    }

    pub fn with_softs(mut self, softs: Vec<(CtrlChg, CtrlChg)>) -> Self {
        self.softs = softs;
        self
    }
}

#[cfg(test)]
mod clipboard_tests {
    use crate::{models::{Models, FromClipboardTextErr}, note::Note, pitch::Pitch, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, duration::{self, Duration, Dots}, velocity::Velocity, trimmer::{Trimmer, RateTrimmer}, bar::{Bar, DcFine, EndOrRegion, RepeatStart}, tempo::Tempo, ctrl_chg::CtrlChg};

    #[test]
    fn parse_empty() {
        assert_eq!(Models::from_clipboard_text("".to_owned()), Err(FromClipboardTextErr::EmptyString));
        assert_eq!(Models::from_clipboard_text(" ".to_owned()), Err(FromClipboardTextErr::EmptyString));
    }

    #[test]
    fn cannot_parse_version() {
        assert_eq!(Models::from_clipboard_text("-1".to_owned()), Err(FromClipboardTextErr::VersionNotU64 { err_json: "-1".to_owned()}));
        assert_eq!(Models::from_clipboard_text("1.1".to_owned()), Err(FromClipboardTextErr::VersionNotU64 { err_json: "1.1".to_owned() }));
        assert_eq!(Models::from_clipboard_text("[0]".to_owned()), Err(FromClipboardTextErr::VersionNotU64 { err_json: "[0]".to_owned() }));

        if let Err(FromClipboardTextErr::CannotParse { err_json: json, detail: _ }) = Models::from_clipboard_text("a".to_owned()) {
            assert_eq!(json, "a".to_owned());
        } else {
            panic!("Logic error.");
        }
    }

    #[test]
    fn version_error() {
        let ver_str = (Models::VERSION + 1).to_string();

        assert_eq!(Models::from_clipboard_text(ver_str), Err(FromClipboardTextErr::VersionErr { detected_ver: Models::VERSION + 1 }));
    }

    #[test]
    fn normal_case() {
        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        let note = Note::new(
            100, pitch,
            Duration::new(duration::Numerator::Quarter, duration::Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO
        );

        let bar = Bar::new(
            100,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );

        let models = Models {
            notes: vec![note], bars: vec![bar], tempos: vec![], dumpers: vec![], softs: vec![]
        };

        let json = models.to_clipboard_text();

        let restored = Models::from_clipboard_text(json).unwrap();
        assert_eq!(restored, models);
    }

    #[test]
    fn move_to_tick() {
        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        let note = Note::new(
            100, pitch,
            Duration::new(duration::Numerator::Quarter, duration::Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO
        );

        let bar = Bar::new(
            110,
            None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );

        let tempo0 = Tempo::new(110, 200);
        let tempo1 = Tempo::new(114, 205);
            
        let dumper = CtrlChg::new(120, Velocity::new(64));

        let soft = CtrlChg::new(90, Velocity::new(64));

        let mut models = Models {
            notes: vec![note],
            bars: vec![bar],
            tempos: vec![tempo0, tempo1],
            dumpers: vec![dumper],
            softs: vec![soft]
        }.move_to_tick(50);

        assert_eq!(models.notes[0].base_start_tick, 60);
        assert_eq!(models.bars[0].start_tick, 70);
        assert_eq!(models.tempos[0].start_tick, 70);
        assert_eq!(models.tempos[1].start_tick, 74);
        assert_eq!(models.dumpers[0].start_tick, 80);
        assert_eq!(models.softs[0].start_tick, 50);
    }
}