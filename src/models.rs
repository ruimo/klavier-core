use std::rc::Rc;

use crate::{note::Note, bar::Bar, tempo::Tempo, ctrl_chg::CtrlChg};

#[derive(Clone, PartialEq, Debug, serde::Deserialize, serde::Serialize)]
pub struct Models {
    pub notes: Vec<Note>,
    pub bars: Vec<Bar>,
    pub tempos: Vec<Tempo>,
    pub dumpers: Vec<CtrlChg>,
    pub softs: Vec<CtrlChg>,
}

impl Models {
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
