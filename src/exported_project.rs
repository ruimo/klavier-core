use crate::{bar::Bar, ctrl_chg::CtrlChg, key::Key, note::Note, rhythm::Rhythm, tempo::Tempo};

#[derive(Clone, PartialEq, Debug, serde::Deserialize, serde::Serialize)]
pub struct ExportedProject {
    pub key: Option<Key>,
    pub rhythm: Option<Rhythm>,
    pub notes: Vec<Note>,
    pub bars: Vec<Bar>,
    pub tempos: Vec<Tempo>,
    pub dumpers: Vec<CtrlChg>,
    pub softs: Vec<CtrlChg>,
}
