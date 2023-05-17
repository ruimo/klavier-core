use std::rc::Rc;

use crate::{note::Note, bar::Bar, tempo::Tempo, ctrl_chg::CtrlChg};

#[derive(Clone, PartialEq, Debug)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Models {
    pub notes: Vec<Rc<Note>>,
    pub bars: Vec<Bar>,
    pub tempos: Vec<Tempo>,
    pub dumpers: Vec<CtrlChg>,
    pub softs: Vec<CtrlChg>,
}

impl Models {
    pub fn empty() -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        }
    }

    pub fn note_only(notes: Vec<Rc<Note>>) -> Self {
        Self {
            notes,
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        }
    }

    pub fn bar_only(bars: Vec<Bar>) -> Self {
        Self {
            notes: vec![],
            bars,
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        }
    }

    pub fn tempo_only(tempos: Vec<Tempo>) -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos,
            dumpers: vec![],
            softs: vec![],
        }
    }

    pub fn dumper_only(dumpers: Vec<CtrlChg>) -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers, 
            softs: vec![],
        }
    }

    pub fn soft_only(softs: Vec<CtrlChg>) -> Self {
        Self {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs,
        }
    }
}