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