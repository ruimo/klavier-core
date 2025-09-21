use std::rc::Rc;

use klavier_helper::bag_store::{BagStore, BagStoreEvent};
use klavier_helper::store::{Store, StoreEvent};
use serde::{Serialize, Deserialize};
use serdo::undo_store::{SqliteUndoStore, UndoStore};
use serdo::cmd::{SerializableCmd, Cmd};

use crate::bar::{Bar, RepeatSet};
use crate::ctrl_chg::CtrlChg;
use crate::grid::Grid;
use crate::key::Key;
use crate::location::Location;
use crate::models::{Models, ModelChanges};
use crate::note::Note;
use crate::rhythm::Rhythm;
use crate::tempo::{TempoValue, Tempo};
use crate::tuple;
use crate::velocity::{Velocity, self};

const DEFAULT_TEMPO: TempoValue = TempoValue::new(120);
const DEFAULT_CTRL_CHG: Velocity = velocity::MIN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationError {
    Overflow,
}

impl std::fmt::Display for LocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => write!(f, "Location overflow"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ChangeRepoType {
    MoveSelected,
    AdHoc,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct ModelChangeMetadata {
    /// None: Do not care.
    /// Some(true): Need to select.
    /// Some(false): Need to unselect.
    pub need_select: Option<bool>,
    pub dragged: bool,
}

impl ModelChangeMetadata {
    pub fn new() -> Self {
        Self {
            need_select: None,
            dragged: false,
        }
    }

    pub fn with_need_select(self, need_select: bool) -> Self {
        Self {
            need_select: Some(need_select),
            dragged: self.dragged,
        }
    }
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(from = "ExportedProject", into = "ExportedProject")]
pub struct ProjectImpl {
    rhythm: Rhythm,
    key: Key,
    grid: Grid,
    note_repo: BagStore<u32, Rc<Note>, ModelChangeMetadata>, // by start tick.
    bar_repo: Store<u32, Bar, ModelChangeMetadata>,
    tempo_repo: Store<u32, Tempo, ModelChangeMetadata>,
    dumper_repo: Store<u32, CtrlChg, ModelChangeMetadata>,
    soft_repo: Store<u32, CtrlChg, ModelChangeMetadata>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExportedProject {
    rhythm: Rhythm,
    key: Key,
    grid: Grid,
    models: Models,
}

impl From<ExportedProject> for ProjectImpl {
    fn from(exported: ExportedProject) -> Self {
        let mut note_repo: BagStore<u32, Rc<Note>, ModelChangeMetadata> = BagStore::new(true);
        note_repo.bulk_add(exported.models.notes.into_iter().map(|n| (n.start_tick(), Rc::new(n))).collect(), ModelChangeMetadata::new());

        let mut bar_repo: Store<u32, Bar, ModelChangeMetadata> = Store::new(true);
        bar_repo.bulk_add(exported.models.bars.into_iter().map(|b| (b.start_tick, b)).collect(), ModelChangeMetadata::new());

        let mut tempo_repo: Store<u32, Tempo, ModelChangeMetadata> = Store::new(true);
        tempo_repo.bulk_add(exported.models.tempos.into_iter().map(|t| (t.start_tick, t)).collect(), ModelChangeMetadata::new());

        let mut dumper_repo: Store<u32, CtrlChg, ModelChangeMetadata> = Store::new(true);
        dumper_repo.bulk_add(exported.models.dumpers.into_iter().map(|d| (d.start_tick, d)).collect(), ModelChangeMetadata::new());

        let mut soft_repo: Store<u32, CtrlChg, ModelChangeMetadata> = Store::new(true);
        soft_repo.bulk_add(exported.models.softs.into_iter().map(|s| (s.start_tick, s)).collect(), ModelChangeMetadata::new());

        ProjectImpl {
            rhythm: exported.rhythm,
            key: exported.key,
            grid: exported.grid,
            note_repo, bar_repo, tempo_repo, dumper_repo, soft_repo
        }
    }
}

impl Into<ExportedProject> for ProjectImpl {
    fn into(self) -> ExportedProject {
        let mut notes: Vec<Note> = Vec::with_capacity(self.note_repo.len());
        for (_, n) in self.note_repo.iter() {
            notes.push((**n).clone());
        }

        let mut bars: Vec<Bar> = Vec::with_capacity(self.bar_repo.len());
        for (_, b) in self.bar_repo.iter() {
            bars.push(*b);
        }

        let mut tempos: Vec<Tempo> = Vec::with_capacity(self.tempo_repo.len());
        for (_, t) in self.tempo_repo.iter() {
            tempos.push(*t);
        }

        let mut dumpers: Vec<CtrlChg> = Vec::with_capacity(self.dumper_repo.len());
        for (_, d) in self.dumper_repo.iter() {
            dumpers.push(*d);
        }

        let mut softs: Vec<CtrlChg> = Vec::with_capacity(self.soft_repo.len());
        for (_, s) in self.soft_repo.iter() {
            softs.push(*s);
        }

        ExportedProject {
            rhythm: self.rhythm,
            key: self.key,
            grid: self.grid,
            models: Models { notes, bars, tempos, dumpers, softs }
        }
    }
}

impl ProjectImpl {
    pub fn note_repo(&self) -> &BagStore<u32, Rc<Note>, ModelChangeMetadata> {
        &self.note_repo
    }
    
    pub fn bar_repo(&self) -> &Store<u32, Bar, ModelChangeMetadata> {
        &self.bar_repo
    }
    
    pub fn tempo_repo(&self) -> &Store<u32, Tempo, ModelChangeMetadata> {
        &self.tempo_repo
    }
    
    pub fn dumper_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata> {
        &self.dumper_repo
    }

    pub fn soft_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata> {
        &self.soft_repo
    }
    
    pub fn rhythm(&self) -> Rhythm {
        self.rhythm
    }
    
    pub fn key(&self) -> Key {
        self.key
    }

    pub fn grid(&self) -> Grid {
        self.grid
    }

    // Add bars without posting undo info.
    fn add_bar_internal(&mut self, bar: Bar, select: bool) -> Vec<Bar> {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        self.bar_repo.add(bar.start_tick, bar, metadata).map(|o| vec![o]).unwrap_or(vec![])
    }

    pub fn bar_no(&self, bar: &Bar) -> Option<usize> {
        match self.bar_repo.index(bar.start_tick) {
            Ok(i) => Some(i),
            Err(_) => None,
        }
    }
    
    pub fn tempo_at(&self, tick: u32) -> TempoValue {
        tempo_at(tick, &self.tempo_repo)
    }
    
    pub fn dumper_at(&self, tick: u32) -> Velocity {
        ctrl_chg_at(tick, &self.dumper_repo)
    }
    
    pub fn soft_at(&self, tick: u32) -> Velocity {
        ctrl_chg_at(tick, &self.soft_repo)
    }
    
    pub fn location_to_tick(&self, loc: Location) -> Result<u32, LocationError> {
        if loc.bar_no() == 0 {
            if (u32::MAX as usize) < loc.offset() {
                Err(LocationError::Overflow)
            } else {
                Ok(loc.offset() as u32)
            }
        } else if (u32::MAX as usize) < loc.bar_no() {
            Err(LocationError::Overflow)
        } else {
            if loc.bar_no() <= self.bar_repo.len() {
                let (tick, _) = self.bar_repo[loc.bar_no() - 1];
                let sum = (tick as usize) + loc.offset();
                if (u32::MAX as usize) < sum {
                    Err(LocationError::Overflow)
                } else {
                    Ok(sum as u32)
                }
            } else {
                let tick = match self.last_bar() {
                    None => {
                        loc.bar_no() * (self.rhythm.tick_len() as usize) + loc.offset()
                    },
                    Some((last_bar_no, last_bar)) => {
                        let last_tick = last_bar.start_tick;
                        let tick_len = self.rhythm_at(last_tick).tick_len() as usize;
                        (loc.bar_no() - last_bar_no - 1) * tick_len + loc.offset() + last_bar.start_tick as usize
                    },
                };

                if (u32::MAX as usize) < tick {
                    Err(LocationError::Overflow)
                } else {
                    Ok(tick as u32)
                }
            }
        }
    }
    
    pub fn tick_to_location(&self, tick: u32) -> Location {
        if self.bar_repo.len() == 0 {
            Location::new(0, tick as usize)
        } else {
            match self.bar_repo.find(&tick) {
                Ok(idx) => Location::new(idx + 1, 0),
                Err(idx) =>
                if idx == 0 {
                    Location::new(0, tick as usize)
                } else {
                    let (_, bar) = self.bar_repo[idx - 1];
                    Location::new(idx, (tick - bar.start_tick) as usize)
                },
            }
        }
    }
    
    pub fn rhythm_at(&self, tick: u32) -> Rhythm {
        let idx = match self.bar_repo.index(tick) {
            Ok(t) => t,
            Err(t) => if t == 0 { return self.rhythm } else { t - 1 },
        };
        
        for i in (0..=idx).rev() {
            let bar = self.bar_repo[i].1;
            if let Some(rhythm) = bar.rhythm {
                return rhythm;
            }
        }
        
        self.rhythm
    }
    
    pub fn key_at(&self, tick: u32) -> Key {
        let idx = match self.bar_repo.index(tick) {
            Ok(t) => t,
            Err(t) => if t == 0 { return self.key } else { t - 1 },
        };
        
        for i in (0..=idx).rev() {
            let bar = self.bar_repo[i].1;
            if let Some(key) = bar.key {
                return key;
            }
        }
        
        self.key
    }

    /// Returns bar no(0 offset) and bar.
    #[inline]
    fn last_bar(&self) -> Option<(usize, Bar)> {
        self.bar_repo.peek_last().map(|(_, bar)| (self.bar_repo.len() - 1, bar.clone()))
    }
    
    fn note_max_end_tick(&self) -> Option<u32> {
        if self.note_repo.is_empty() { return None; }
        let mut start_tick =
        self.note_repo.peek_last().map(|t| { t.0.clone() }).unwrap_or(0) as i64 - *Note::LONGEST_TICK_LEN as i64;
        if start_tick < 0 { start_tick = 0; }
        let mut max_tick_loc = 0;
        for (tick, note) in self.note_repo.range(start_tick as u32..) {
            let end_tick = tick + note.tick_len();
            max_tick_loc = max_tick_loc.max(end_tick);
        }
        
        Some(max_tick_loc)
    }
    
    #[inline]
    fn tempo_max_tick(&self) -> Option<u32> {
        self.tempo_repo.peek_last().map(|t| t.0)
    }
    
    #[inline]
    fn dumper_max_tick(&self) -> Option<u32> {
        self.dumper_repo.peek_last().map(|t| t.0)
    }
    
    #[inline]
    fn soft_max_tick(&self) -> Option<u32> {
        self.soft_repo.peek_last().map(|t| t.0)
    }
    
    /// Returns replenished bars.
    fn replenish_bars(&mut self) -> Vec<Bar> {
        let mut bar_tick = self.last_bar().map(|(_, b)| b.start_tick).unwrap_or(0);
        let max_end_tick = 
        self.note_max_end_tick().unwrap_or(0)
        .max(self.tempo_max_tick().unwrap_or(0))
        .max(self.dumper_max_tick().unwrap_or(0))
        .max(self.soft_max_tick().unwrap_or(0));
        
        let mut replenished_bars: Vec<Bar> = vec![];
        if max_end_tick <= bar_tick { return replenished_bars; }
        let bar_tick_len = self.rhythm_at(bar_tick).tick_len();
        while bar_tick < max_end_tick {
            bar_tick += bar_tick_len;
            let bar = Bar::new(bar_tick, None, None, RepeatSet::EMPTY);
            self.add_bar_internal(bar, false);
            replenished_bars.push(bar);
        }
        replenished_bars
    }
}

pub fn tempo_at(tick: u32, store: &Store<u32, Tempo, ModelChangeMetadata>) -> TempoValue {
    if store.is_empty() {
        DEFAULT_TEMPO
    } else {
        match store.find(&tick) {
            Ok(idx) => {
                store[idx].1.value
            },
            Err(idx) => {
                if idx == 0 {
                    DEFAULT_TEMPO
                } else {
                    store[idx - 1].1.value
                }
            },
        }
    }
}

pub fn ctrl_chg_at(tick: u32, store: &Store<u32, CtrlChg, ModelChangeMetadata>) -> Velocity {
    if store.is_empty() {
        DEFAULT_CTRL_CHG
    } else {
        match store.find(&tick) {
            Ok(idx) => {
                store[idx].1.velocity
            },
            Err(idx) => {
                if idx == 0 {
                    DEFAULT_CTRL_CHG
                } else {
                    store[idx - 1].1.velocity
                }
            },
        }
    }
}

impl Default for ProjectImpl {
    fn default() -> Self {
        ProjectImpl {
            rhythm: Rhythm::default(),
            key: Key::NONE,
            grid: Grid::default(),
            note_repo: BagStore::new(true),
            bar_repo: Store::new(true),
            tempo_repo: Store::new(true),
            dumper_repo: Store::new(true),
            soft_repo: Store::new(true),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ProjectCmd {
    SetRhythm(Rhythm, Rhythm),
    SetKey(Key, Key),
    SetGrid(Grid, Grid),
    ModelChanged { added: Models, removed: Models, metadata: ModelChangeMetadata },
}

impl Cmd for ProjectCmd {
    type Model = ProjectImpl;
    
    fn undo(&self, proj: &mut Self::Model) {
        match self {
            ProjectCmd::SetRhythm(old_rhythm, _) => {
                proj.rhythm = *old_rhythm;
            },
            ProjectCmd::SetKey(old_key, _) => {
                proj.key = *old_key;
            },
            ProjectCmd::SetGrid(old_grid, _) => {
                proj.grid = *old_grid;
            },
            ProjectCmd::ModelChanged { added, removed, metadata } => {
                for n in added.notes.iter() {
                    proj.note_repo.remove(&n.start_tick(), &Rc::new((*n).clone()));
                }
                for b in added.bars.iter() {
                    proj.bar_repo.remove(&b.start_tick);
                }
                for t in added.tempos.iter() {
                    proj.tempo_repo.remove(&t.start_tick);
                }
                for d in added.dumpers.iter() {
                    proj.dumper_repo.remove(&d.start_tick);
                }
                for s in added.softs.iter() {
                    proj.soft_repo.remove(&&s.start_tick);
                }
                
                for n in removed.notes.iter() {
                    proj.note_repo.add(n.start_tick(), Rc::new((*n).clone()), *metadata);
                }
                for b in removed.bars.iter() {
                    proj.bar_repo.add(b.start_tick, *b, *metadata);
                }
                for t in removed.tempos.iter() {
                    proj.tempo_repo.add(t.start_tick, *t, *metadata);
                }
                for d in removed.dumpers.iter() {
                    proj.dumper_repo.add(d.start_tick, *d, *metadata);
                }
                for s in removed.softs.iter() {
                    proj.soft_repo.add(s.start_tick, *s, *metadata);
                }
            },
        }
    }
    
    fn redo(&self, proj: &mut Self::Model) {
        match self {
            ProjectCmd::SetRhythm(_, new_rhythm) => {
                proj.rhythm = *new_rhythm;
            },
            ProjectCmd::SetKey(_, new_key) => {
                proj.key = *new_key;
            },
            ProjectCmd::SetGrid(_, new_grid) => {
                proj.grid = *new_grid;
            }
            ProjectCmd::ModelChanged { added, removed , metadata } => {
                for n in removed.notes.iter() {
                    proj.note_repo.remove(&n.start_tick(), &Rc::new(n.clone()));
                }
                for b in removed.bars.iter() {
                    proj.bar_repo.remove(&b.start_tick);
                }
                for t in removed.tempos.iter() {
                    proj.tempo_repo.remove(&t.start_tick);
                }
                for d in removed.dumpers.iter() {
                    proj.dumper_repo.remove(&d.start_tick);
                }
                for s in removed.softs.iter() {
                    proj.soft_repo.remove(&&s.start_tick);
                }
                
                for n in added.notes.iter() {
                    proj.note_repo.add(n.start_tick(), Rc::new(n.clone()), *metadata);
                }
                for b in added.bars.iter() {
                    proj.bar_repo.add(b.start_tick, *b, *metadata);
                }
                for t in added.tempos.iter() {
                    proj.tempo_repo.add(t.start_tick, *t, *metadata);
                }
                for d in added.dumpers.iter() {
                    proj.dumper_repo.add(d.start_tick, *d, *metadata);
                }
                for s in added.softs.iter() {
                    proj.soft_repo.add(s.start_tick, *s, *metadata);
                }
            },
        }
    }
}

impl SerializableCmd for ProjectCmd {
}

#[derive(Debug)]
pub enum ProjectCmdErr {
    NoOp,
}

impl core::error::Error for ProjectCmdErr {}

impl std::fmt::Display for ProjectCmdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectCmdErr::NoOp => write!(f, "No Operation"),
        }
    }
}

pub trait Project {
    fn set_rhythm(&mut self, rhythm: Rhythm);
    fn rhythm(&self) -> Rhythm;
    fn set_key(&mut self, key: Key);
    fn key(&self) -> Key;
    fn set_grid(&mut self, key: Grid);
    fn grid(&self) -> Grid;
    fn add_note(&mut self, note: Note, select: bool);
    fn add_bar(&mut self, bar: Bar, select: bool);
    fn add_tempo(&mut self, bar: Tempo, select: bool);
    fn add_dumper(&mut self, dumper: CtrlChg, select: bool);
    fn add_soft(&mut self, soft: CtrlChg, select: bool);
    fn tuplize(&mut self, notes: Vec<Rc<Note>>);
    fn bulk_remove(&mut self, to_remove: Models, metadata: ModelChangeMetadata);
    fn bulk_add(&mut self, to_add: Models, metadata: ModelChangeMetadata);
    fn change(&mut self, from_to: ModelChanges, metadata: ModelChangeMetadata);
    fn bar_no(&self, bar: &Bar) -> Option<usize>;
    fn tempo_at(&self, tick: u32) -> TempoValue;
    fn dumper_at(&self, tick: u32) -> Velocity;
    fn soft_at(&self, tick: u32) -> Velocity;
    fn clear_model_events(&mut self);
    fn bar_events(&self) -> &Vec<StoreEvent<u32, Bar, ModelChangeMetadata>>;
    fn tempo_events(&self) -> &Vec<StoreEvent<u32, Tempo, ModelChangeMetadata>>;
    fn dumper_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>>;
    fn soft_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>>;
    fn note_events(&self) -> &Vec<BagStoreEvent<u32, Rc<Note>, ModelChangeMetadata>>;
    fn location_to_tick(&self, loc: Location) -> Result<u32, LocationError>;
    fn tick_to_location(&self, tick: u32) -> Location;
    fn rhythm_at(&self, tick: u32) -> Rhythm;
    fn key_at(&self, tick: u32) -> Key;
    fn note_repo(&self) -> &BagStore<u32, Rc<Note>, ModelChangeMetadata>;
    fn bar_repo(&self) -> &Store<u32, Bar, ModelChangeMetadata>;
    fn tempo_repo(&self) -> &Store<u32, Tempo, ModelChangeMetadata>;
    fn soft_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata>;
    fn dumper_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata>;
}

impl Project for SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr> {
    fn set_rhythm(&mut self, rhythm: Rhythm) {
        self.add_cmd(ProjectCmd::SetRhythm(self.model().rhythm, rhythm));
    }
    
    fn rhythm(&self) -> Rhythm {
        self.model().rhythm
    }

    fn set_key(&mut self, key: Key) {
        self.add_cmd(ProjectCmd::SetKey(self.model().key, key));
    }

    fn key(&self) -> Key {
        self.model().key
    }
    
    fn set_grid(&mut self, grid: Grid) {
        self.add_cmd(ProjectCmd::SetGrid(self.model().grid, grid));
    }

    fn grid(&self) -> Grid {
        self.model().grid
    }
    
    fn add_note(&mut self, note: Note, select: bool) {
        let note = Rc::new(note);
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }

        let _ = self.mutate(Box::new(move |proj| {
            proj.note_repo.add(note.start_tick(), note.clone(), metadata);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_notes(&[note.clone()]).with_bars(replenishid_bars),
                    removed: Models::empty(),
                    metadata,
                }
            )
        }));
    }
    
    fn add_bar(&mut self, bar: Bar, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(Box::new(move |proj| {
            let origin = proj.bar_repo.add(bar.start_tick, bar, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(vec![bar]),
                    removed: Models::empty().with_bars(origin),
                    metadata,
                }
            )
        }));
    }
    
    fn add_tempo(&mut self, tempo: Tempo, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(Box::new(move |proj| {
            let origin = proj.tempo_repo.add(tempo.start_tick, tempo, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_tempos(vec![tempo]),
                    removed: Models::empty().with_tempos(origin),
                    metadata,
                }
                
            )
        }));
    }
    
    fn add_dumper(&mut self, dumper: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(Box::new(move |proj| {
            let origin = proj.dumper_repo.add(dumper.start_tick, dumper, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_dumpers(vec![dumper]),
                    removed: Models::empty().with_dumpers(origin),
                    metadata,
                }
            )
        }));
    }
    
    fn add_soft(&mut self, soft: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(Box::new(move |proj| {
            let origin = proj.soft_repo.add(soft.start_tick, soft, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_softs(vec![soft]),
                    removed: Models::empty().with_softs(origin),
                    metadata
                }
            )
        }));
    }
    
    fn tuplize(&mut self, notes: Vec<Rc<Note>>) {
        let metadata = ModelChangeMetadata::new().with_need_select(true);
        let _ = self.mutate(Box::new(move |proj| {
            if 1 < notes.len() {
                let mut to_remove = Vec::with_capacity(notes.len());
                for n in notes.iter() {
                    to_remove.push((n.start_tick(), n.clone()));
                }
                let tupled = tuple::tuplize(notes.clone());
                proj.note_repo.bulk_remove(&to_remove, ModelChangeMetadata::new());
                proj.note_repo.bulk_add(
                    tupled.iter().map(|n| (n.start_tick(), n.clone())).collect(),
                    metadata
                );
                let replenishid_bars = proj.replenish_bars();
                
                Ok(
                    ProjectCmd::ModelChanged {
                        added: Models::empty().with_notes(&tupled).with_bars(replenishid_bars),
                        removed: Models::empty().with_notes(&notes),
                        metadata,
                    }
                )
            } else {
                Err(error_stack::report!(ProjectCmdErr::NoOp))
            }
        }));
        
    }

    fn bulk_remove(&mut self, to_remove: Models, metadata: ModelChangeMetadata) {
        self.add_cmd(ProjectCmd::ModelChanged { added: Models::empty(), removed: to_remove, metadata });
    }

    fn bulk_add(&mut self, mut to_add: Models, metadata: ModelChangeMetadata) {
        let _ = self.mutate(Box::new(move |proj| {
            let mut removed = Models::empty();

            let mut buf: Vec<(u32, Rc<Note>)> = Vec::with_capacity(to_add.notes.len());
            for n in to_add.notes.iter() {
                buf.push((n.start_tick(), Rc::new(n.clone())));
            }   
            proj.note_repo.bulk_add(buf, metadata);
    
            let mut buf = Vec::with_capacity(to_add.bars.len());
            for b in to_add.bars.iter() {
                buf.push((b.start_tick, *b));
            }
            removed.bars = proj.bar_repo.bulk_add(buf, metadata).iter().map(|(_, bar)| *bar).collect();
    
            let mut buf = Vec::with_capacity(to_add.tempos.len());
            for t in to_add.tempos.iter() {
                buf.push((t.start_tick, *t));
            }
            removed.tempos = proj.tempo_repo.bulk_add(buf, metadata).iter().map(|(_, t)| *t).collect();
    
    
            let mut buf = Vec::with_capacity(to_add.dumpers.len());
            for d in to_add.dumpers.iter() {
                buf.push((d.start_tick, *d));
            }
            removed.dumpers = proj.dumper_repo.bulk_add(buf, metadata).iter().map(|(_, d)| *d).collect();
    
            let mut buf = Vec::with_capacity(to_add.softs.len());
            for s in to_add.softs.iter() {
                buf.push((s.start_tick, *s));
            }
            removed.softs = proj.soft_repo.bulk_add(buf, metadata).iter().map(|(_, s)| *s).collect();
            
            let replenished_bars = proj.replenish_bars();
            to_add.bars.extend(replenished_bars);
    
            Ok(ProjectCmd::ModelChanged { added: to_add, removed, metadata })
        }));
    }

    fn change(&mut self, from_to: ModelChanges, metadata: ModelChangeMetadata) {
        let _ = self.mutate(Box::new(move |proj| {
            let mut added: Models = Models::with_capacity(
                from_to.notes.len(),
                from_to.bars.len(),
                from_to.tempos.len(),
                from_to.dumpers.len(),
                from_to.softs.len(),
            );

            let mut removed: Models = Models::with_capacity(
                from_to.notes.len(),
                from_to.bars.len(),
                from_to.tempos.len(),
                from_to.dumpers.len(),
                from_to.softs.len(),
            );

            let mut note_change: Vec<((u32, Rc<Note>), (u32, Rc<Note>))> = Vec::with_capacity(from_to.notes.len());
            for (from, to) in from_to.notes.iter() {
                note_change.push((
                    (from.start_tick(), Rc::new(from.clone())), (to.start_tick(), Rc::new(to.clone()))
                ));
                added.notes.push(to.clone());
                removed.notes.push(from.clone());
            }
            proj.note_repo.change(&note_change, metadata);

            let mut bar_change: Vec<(&u32, (u32, Bar))> = Vec::with_capacity(from_to.bars.len());
            for (from, to) in from_to.bars.iter() {
                bar_change.push((
                    &from.start_tick, (to.start_tick, *to)
                ));
                added.bars.push(*to);
                removed.bars.push(*from);
            }
            removed.bars.extend(proj.bar_repo.change(&bar_change, metadata).iter().map(|(_, b)| *b).collect::<Vec<Bar>>());

            let mut tempo_change: Vec<(&u32, (u32, Tempo))> = Vec::with_capacity(from_to.tempos.len());
            for (from, to) in from_to.tempos.iter() {
                tempo_change.push((
                    &from.start_tick, (to.start_tick, *to)
                ));
                added.tempos.push(*to);
                removed.tempos.push(*from);
            }
            removed.tempos.extend(proj.tempo_repo.change(&tempo_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<Tempo>>());

            let mut dumper_change: Vec<(&u32, (u32, CtrlChg))> = Vec::with_capacity(from_to.dumpers.len());
            for (from, to) in from_to.dumpers.iter() {
                dumper_change.push((
                    &from.start_tick, (to.start_tick, *to)
                ));
                added.dumpers.push(*to);
                removed.dumpers.push(*from);
            }
            removed.dumpers.extend(proj.dumper_repo.change(&dumper_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<CtrlChg>>());

            let mut soft_change: Vec<(&u32, (u32, CtrlChg))> = Vec::with_capacity(from_to.softs.len());
            for (from, to) in from_to.softs.iter() {
                soft_change.push((
                    &from.start_tick, (to.start_tick, *to)
                ));
                added.softs.push(*to);
                removed.softs.push(*from);
            }
            removed.softs.extend(proj.soft_repo.change(&soft_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<CtrlChg>>());

            added.bars.extend(proj.replenish_bars());
            Ok(ProjectCmd::ModelChanged {
                added, removed, metadata
            })
        }));
    }

    #[inline]
    fn bar_no(&self, bar: &Bar) -> Option<usize> {
        self.model().bar_no(bar)
    }

    #[inline]
    fn tempo_at(&self, tick: u32) -> TempoValue {
        self.model().tempo_at(tick)
    }

    #[inline]
    fn dumper_at(&self, tick: u32) -> Velocity {
        self.model().dumper_at(tick)
    }

    #[inline]
    fn soft_at(&self, tick: u32) -> Velocity {
        self.model().soft_at(tick)
    }

    fn clear_model_events(&mut self) {
        let _ = self.irreversible_mutate(Box::new(|proj| {
            proj.note_repo.clear_events();
            proj.bar_repo.clear_events();
            proj.tempo_repo.clear_events();
            proj.dumper_repo.clear_events();
            proj.soft_repo.clear_events();
        }));
    }

    #[inline]
    fn bar_events(&self) -> &Vec<StoreEvent<u32, Bar, ModelChangeMetadata>> {
        self.model().bar_repo.events()
    }

    #[inline]
    fn tempo_events(&self) -> &Vec<StoreEvent<u32, Tempo, ModelChangeMetadata>> {
        self.model().tempo_repo.events()
    }

    #[inline]
    fn dumper_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>> {
        self.model().dumper_repo.events()

    }

    #[inline]
    fn soft_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>> {
        self.model().soft_repo.events()
    }

    #[inline]
    fn note_events(&self) -> &Vec<BagStoreEvent<u32, Rc<Note>, ModelChangeMetadata>> {
        self.model().note_repo.events()
    }

    #[inline]
    fn location_to_tick(&self, loc: Location) -> Result<u32, LocationError> {
        self.model().location_to_tick(loc)
    }

    #[inline]
    fn tick_to_location(&self, tick: u32) -> Location {
        self.model().tick_to_location(tick)
    }

    #[inline]
    fn rhythm_at(&self, tick: u32) -> Rhythm {
        self.model().rhythm_at(tick)
    }

    #[inline]
    fn key_at(&self, tick: u32) -> Key {
        self.model().key_at(tick)
    }

    #[inline]
    fn note_repo(&self) -> &BagStore<u32, Rc<Note>, ModelChangeMetadata> {
        &self.model().note_repo()
    }

    #[inline]
    fn bar_repo(&self) -> &Store<u32, Bar, ModelChangeMetadata> {
        self.model().bar_repo()
    }

    #[inline]
    fn tempo_repo(&self) -> &Store<u32, Tempo, ModelChangeMetadata> {
        self.model().tempo_repo()
    }

    #[inline]
    fn soft_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata> {
        self.model().soft_repo()
    }

    #[inline]
    fn dumper_repo(&self) -> &Store<u32, CtrlChg, ModelChangeMetadata> {
        self.model().dumper_repo()
    }
}   

pub type ProjectStore = SqliteUndoStore<ProjectCmd, ProjectImpl, ProjectCmdErr>;

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use klavier_helper::store::Store;
    use serdo::undo_store::{SqliteUndoStore, UndoStore, self};
    use crate::{tempo::{Tempo, TempoValue}, project::{tempo_at, ProjectCmd, ProjectCmdErr, ModelChangeMetadata, ProjectStore, LocationError}, note::Note, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, pitch::Pitch, duration::{Duration, Numerator, Denominator, Dots}, velocity::Velocity, trimmer::{Trimmer, RateTrimmer}, bar::{Bar, RepeatSet}, location::Location, rhythm::Rhythm, ctrl_chg::CtrlChg, key::Key, grid::Grid, models::{Models, ModelChanges}, channel::Channel};
    use super::{DEFAULT_TEMPO, ProjectImpl};

    #[test]
    fn tempo() {
        let mut store: Store<u32, Tempo, ModelChangeMetadata> = Store::new(false);
        assert_eq!(tempo_at(0, &store), DEFAULT_TEMPO);
        let metadata = ModelChangeMetadata::new();

        store.add(10, Tempo { start_tick: 10, value: TempoValue::new(100) }, metadata);
        assert_eq!(tempo_at(0, &store), DEFAULT_TEMPO);
        assert_eq!(tempo_at(10, &store), TempoValue::new(100));
        assert_eq!(tempo_at(11, &store), TempoValue::new(100));
        
        store.add(20, Tempo { start_tick: 20, value: TempoValue::new(200) }, metadata);
        assert_eq!(tempo_at(0, &store), DEFAULT_TEMPO);
        assert_eq!(tempo_at(10, &store), TempoValue::new(100));
        assert_eq!(tempo_at(11, &store), TempoValue::new(100));
        assert_eq!(tempo_at(19, &store), TempoValue::new(100));
        assert_eq!(tempo_at(20, &store), TempoValue::new(200));
        assert_eq!(tempo_at(100, &store), TempoValue::new(200));
    }
    
    #[test]
    fn undo_note_addition() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store: ProjectStore = ProjectStore::open(dir.clone(), undo_store::Options::new()).unwrap();

        let note = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false,
            false,
            Velocity::new(64),
            Trimmer::ZERO,
            RateTrimmer::ONE,
            Trimmer::ZERO,
            Channel::default(),
        );
        
        store.add_note(note, false);
        assert_eq!(store.model().note_repo().len(), 1);
        
        store.undo();
        assert_eq!(store.model().note_repo().len(), 0);
    }
    
    #[test]
    fn undo_bar_addition() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store: ProjectStore = ProjectStore::open(dir.clone(), undo_store::Options::new()).unwrap();

        let bar = Bar::new(
            100,
            None,
            None,
            RepeatSet::EMPTY,
        );
        store.add_bar(bar, false);
        assert_eq!(store.model().bar_repo().len(), 1);
        store.undo();
        assert_eq!(store.model().bar_repo().len(), 0);
    }
    
    #[test]
    fn location_to_tick() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        assert_eq!(store.model().location_to_tick(Location::new(0, 0)).unwrap(), 0);
        assert_eq!(store.model().location_to_tick(Location::new(0, 1)).unwrap(), 1);
        assert_eq!(store.model().location_to_tick(Location::new(0, u32::MAX as usize)).unwrap(), u32::MAX);
        assert_eq!(store.model().location_to_tick(Location::new(0, u32::MAX as usize + 1)), Err(LocationError::Overflow));
        assert_eq!(
            store.model().location_to_tick(Location::new(1, 1)).unwrap(),
            store.model().rhythm().tick_len() + 1
        );
        
        let bar0 = Bar::new(
            100, None, None, RepeatSet::EMPTY
        );
        store.add_bar(bar0, false);
        
        assert_eq!(store.model().location_to_tick(Location::new(0, 0)).unwrap(), 0);
        assert_eq!(store.model().location_to_tick(Location::new(0, 1)).unwrap(), 1);
        assert_eq!(store.model().location_to_tick(Location::new(1, 1)).unwrap(), 101);
        assert_eq!(store.model().location_to_tick(Location::new(1, (u32::MAX as usize) - 100)).unwrap(), u32::MAX);
        assert_eq!(store.model().location_to_tick(Location::new(1, (u32::MAX as usize) - 99)), Err(LocationError::Overflow));

        // 0   bar0(t=100)
        //     |
        assert_eq!(store.model().location_to_tick(Location::new(2, 1)).unwrap(), 100 + store.model().rhythm().tick_len() + 1);
        
        let bar1 = Bar::new(
            1000, None, None, RepeatSet::EMPTY
        );
        store.add_bar(bar1, false);
        
        assert_eq!(store.model().location_to_tick(Location::new(0, 0)).unwrap(), 0);
        assert_eq!(store.model().location_to_tick(Location::new(0, 1)).unwrap(), 1);
        assert_eq!(store.model().location_to_tick(Location::new(1, 1)).unwrap(), 101);
        assert_eq!(store.model().location_to_tick(Location::new(2, 0)).unwrap(), 1000);

        let bar2 = Bar::new(
            2000, Some(Rhythm::new(2, 4)), None, RepeatSet::EMPTY
        );
        store.add_bar(bar2, false);

        let bar3 = Bar::new(
            3000, None, None, RepeatSet::EMPTY
        );
        store.add_bar(bar3, false);

        // 0   bar0(t=100)  bar1(t=1000) bar2(t=2000 r=2/4) bar3(t = 3000)
        //     |            |            |                  |
        assert_eq!(store.model().location_to_tick(Location::new(4, 1)).unwrap(), 3001);
        assert_eq!(store.model().location_to_tick(Location::new(5, 1)).unwrap(), 3000 + 480 + 1);

    }
    
    #[test]
    fn tick_to_location() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        assert_eq!(store.model().tick_to_location(0), Location::new(0, 0));
        assert_eq!(store.model().tick_to_location(100), Location::new(0, 100));
        assert_eq!(store.model().tick_to_location(u32::MAX), Location::new(0, u32::MAX as usize));
        
        let bar = Bar::new(
            100, None, None, RepeatSet::EMPTY
        );
        store.add_bar(bar, false);
        
        assert_eq!(store.model().tick_to_location(0), Location::new(0, 0));
        assert_eq!(store.model().tick_to_location(99), Location::new(0, 99));
        assert_eq!(store.model().tick_to_location(100), Location::new(1, 0));
        assert_eq!(store.model().tick_to_location(u32::MAX), Location::new(1, (u32::MAX - 100) as usize));
        
        let bar = Bar::new(
            1000, None, None, RepeatSet::EMPTY,
        );
        store.add_bar(bar, false);
        
        assert_eq!(store.model().tick_to_location(0), Location::new(0, 0));
        assert_eq!(store.model().tick_to_location(99), Location::new(0, 99));
        assert_eq!(store.model().tick_to_location(999), Location::new(1, 899));
        assert_eq!(store.model().tick_to_location(1000), Location::new(2, 0));
        assert_eq!(store.model().tick_to_location(u32::MAX), Location::new(2, (u32::MAX - 1000) as usize));
    }
    
    #[test]
    fn rhythm_at() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        store.set_rhythm(Rhythm::new(6, 8));
        assert_eq!(store.model().last_bar(), None);
        
        assert_eq!(store.model().rhythm_at(500), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(0), Rhythm::new(6, 8));
        
        let bar0 = Bar::new(100, None, None, RepeatSet::EMPTY);
        store.add_bar(bar0, false);
        assert_eq!(store.model().last_bar().map(|(_, bar)| bar), Some(bar0));
        
        assert_eq!(store.model().rhythm_at(0), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(99), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(100), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(101), Rhythm::new(6, 8));
        
        let bar1 = Bar::new(
            200, Some(Rhythm::new(3, 4)), None, RepeatSet::EMPTY
        );
        store.add_bar(bar1, false);
        assert_eq!(store.model().last_bar().map(|(_, bar)| bar), Some(bar1));
        
        let bar2 = Bar::new(300, None, None, RepeatSet::EMPTY);
        store.add_bar(bar2, false);
        assert_eq!(store.model().last_bar().map(|(_, bar)| bar), Some(bar2));
        
        let bar3 = Bar::new(
            400, Some(Rhythm::new(4, 4)), None, RepeatSet::EMPTY
        );
        store.add_bar(bar3, false);
        assert_eq!(store.model().last_bar().map(|(_, bar)| bar), Some(bar3));
        
        assert_eq!(store.model().rhythm_at(0), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(99), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(100), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(101), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(199), Rhythm::new(6, 8));
        assert_eq!(store.model().rhythm_at(200), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(201), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(299), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(300), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(301), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(399), Rhythm::new(3, 4));
        assert_eq!(store.model().rhythm_at(400), Rhythm::new(4, 4));
        assert_eq!(store.model().rhythm_at(401), Rhythm::new(4, 4));
    }
    
    #[test]
    fn note_max_tick_loc() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        assert_eq!(store.model().note_max_end_tick(), None);
        
        let note0 = Note::new( // end tick: 100 + 240 * 1.5 = 460
            100, pitch,
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO,
            Channel::default(),
        );
        
        let end_tick0 = note0.base_start_tick + note0.tick_len();
        store.add_note(note0, false);
        assert_eq!(store.model().note_max_end_tick(), Some(end_tick0));
        
        let note1 = Note::new( // end tick: 100 + 960 * 1.5 = 1540
            100, pitch,
            Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO,
            Channel::default(),
        );
        
        store.add_note(note1.clone(), false);
        assert_eq!(store.model().note_max_end_tick(), Some(note1.base_start_tick + note1.tick_len()));
        
        let _ = store.add_note(
            Note::new( // end tick: 200 + 120 * 2.5 = 440
                200, pitch,
                Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 2.0),
                Trimmer::ZERO,
                Channel::default(),
            ), false
        );
        
        assert_eq!(store.model().note_max_end_tick(), Some(note1.base_start_tick + note1.tick_len()));
    }
    
    #[test]
    fn replenish_bars() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        assert_eq!(store.model().bar_repo().len(), 0);
        store.add_note(
            Note::new( // end tick: 100 + 240 * 1.5 = 460
                100, pitch,
                Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO,
                Channel::default(),
            ), false
        );
        assert_eq!(store.model().bar_repo().len(), 1);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960);
        
        store.add_note(
            Note::new(
                0, pitch,
                Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO,
                Channel::default(),
            ), false
        );

        assert_eq!(store.model().bar_repo().len(), 1);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960);
        
        store.add_note(
            Note::new(
                960, pitch,
                Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO,
                Channel::default(),
            ), false
        );

        assert_eq!(store.model().bar_repo().len(), 3);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 3);
        
        store.add_tempo(Tempo::new(960*3, 200), false);

        assert_eq!(store.model().bar_repo().len(), 3);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 3);
        
        store.add_tempo(Tempo::new(960 * 3 + 1, 200), false);

        assert_eq!(store.model().bar_repo().len(), 4);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 4);
        
        store.add_dumper(CtrlChg::new(960 * 4, Velocity::new(20), Channel::default()), false);

        assert_eq!(store.model().bar_repo().len(), 4);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 4);
        
        store.add_dumper(CtrlChg::new(960 * 4 + 1, Velocity::new(20), Channel::default()), false);

        assert_eq!(store.model().bar_repo().len(), 5);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 5);
        
        store.add_soft(CtrlChg::new(960 * 5, Velocity::new(20), Channel::default()), false);

        assert_eq!(store.model().bar_repo().len(), 5);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 5);
        
        store.add_dumper(CtrlChg::new(960 * 5 + 1, Velocity::new(20), Channel::default()), false);

        assert_eq!(store.model().bar_repo().len(), 6);
        assert_eq!(store.model().bar_repo().peek_last().unwrap().0, 960 * 6);
    }
    
    #[test]
    fn tuplize() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();

        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note0.clone(), false);
        
        let note1 = Note::new(
            120,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note1.clone(), false);
        
        let note2 = Note::new(
            150,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note2.clone(), false);
        assert_eq!(store.model().note_repo().len(), 3);
        
        store.tuplize(vec![Rc::new(note0), Rc::new(note1), Rc::new(note2)]);
        assert_eq!(store.model().note_repo().len(), 3);
        
        let mut z = store.model().note_repo().iter();
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100 + 80);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100 + 80 * 2);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        store.undo();
        assert_eq!(store.model().note_repo().len(), 3);
        
        let mut z = store.model().note_repo().iter();
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 120);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 150);
        assert_eq!(note.duration, Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
    }
    
    #[test]
    fn can_serialize_project() {
        let mut proj = ProjectImpl::default();
        proj.key = Key::FLAT_2;
        proj.rhythm = Rhythm::new(3, 4);
        
        let note0 = Rc::new(Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        ));
        proj.note_repo.add(note0.start_tick(), note0.clone(), ModelChangeMetadata::new());

        let ser = bincode::serialize(&proj).unwrap();
        
        let des: ProjectImpl = bincode::deserialize(&ser).unwrap();
        
        assert_eq!(proj.key, des.key);
        assert_eq!(proj.rhythm, des.rhythm);
        assert_eq!(des.note_repo.len(), 1);
        assert_eq!(*des.note_repo.iter().map(|(_, n)| n).next().unwrap(), note0);
    }
    
    use tempfile::tempdir;
    use super::Project;
    
    #[test]
    fn can_undo_set_rhythm() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        store.set_rhythm(Rhythm::new(12, 8));
        store.wait_until_saved();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.set_rhythm(Rhythm::new(12, 4));
        store.wait_until_saved();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 4));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::default());
        
        store.redo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.set_rhythm(Rhythm::new(16, 8));
        store.wait_until_saved();
        assert_eq!(store.model().rhythm(), Rhythm::new(16, 8));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
    }
    
    #[test]
    fn can_undo_set_key() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        store.set_key(Key::FLAT_1);
        store.set_key(Key::FLAT_2);
        store.wait_until_saved();
        assert_eq!(store.model().key, Key::FLAT_2);
        
        store.undo();
        assert_eq!(store.model().key, Key::FLAT_1);
        
        store.redo();
        assert_eq!(store.model().key, Key::FLAT_2);
    }
    
    #[test]
    fn can_undo_set_grid() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        store.set_grid(Grid::from_u32(100).unwrap());
        store.set_grid(Grid::from_u32(200).unwrap());
        store.wait_until_saved();        
        assert_eq!(store.model().grid.as_u32(), 200);
        
        store.undo();
        assert_eq!(store.model().grid.as_u32(), 100);
        
        store.redo();
        assert_eq!(store.model().grid.as_u32(), 200);
    }

    #[test]
    fn can_undo_add_note() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        
        store.add_note(note0.clone(), false);
        store.wait_until_saved();
        assert_eq!(store.model().note_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1); // bar is replenished.
        
        store.undo();
        assert_eq!(store.model().note_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        store.redo();
        assert_eq!(store.model().note_repo.len(), 1);
        assert_eq!(store.model().note_repo.get(100u32), &vec![Rc::new(note0.clone())]);
        assert_eq!(store.model().bar_repo.len(), 1);
    }
    
    #[test]
    fn can_undo_add_bar() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let bar0 = Bar::new(
            1000, Some(Rhythm::new(3, 4)), None, RepeatSet::EMPTY
        );
        
        store.add_bar(bar0, false);
        store.wait_until_saved();
        assert_eq!(store.model().bar_repo.len(), 1); // with replenished bar
        
        store.undo();
        assert_eq!(store.model().bar_repo.len(), 0);
        
        store.redo();
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().bar_repo[0], (1000u32, bar0))
    }
    
    #[test]
    fn can_undo_add_tempo() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let tempo0 = Tempo::new(200, 200);
        
        store.add_tempo(tempo0, false);
        store.wait_until_saved();
        assert_eq!(store.model().tempo_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1); // bar is replenished.
        
        store.undo();
        assert_eq!(store.model().tempo_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        store.redo();
        assert_eq!(store.model().tempo_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().tempo_repo[0], (200u32, tempo0));
    }
    
    #[test]
    fn can_undo_add_dumper() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let dumper = CtrlChg::new(200, Velocity::new(20), Channel::default());
        store.add_dumper(dumper, false);
        store.wait_until_saved();
        assert_eq!(store.model().dumper_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1); // bar is replenished.
        
        store.undo();
        assert_eq!(store.model().dumper_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        store.redo();
        assert_eq!(store.model().dumper_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().dumper_repo[0], (200u32, dumper));
    }
    
    #[test]
    fn can_undo_add_soft() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let soft = CtrlChg::new(200, Velocity::new(20), Channel::default());
        store.add_soft(soft, false);
        store.wait_until_saved();
        assert_eq!(store.model().soft_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1); // bar is replenished.
        
        store.undo();
        assert_eq!(store.model().soft_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        store.redo();
        assert_eq!(store.model().soft_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().soft_repo[0], (200u32, soft));
    }
    
    #[test]
    fn can_undo_tuplize() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        // Do nothing for empty.
        store.tuplize(vec![]);
        assert_eq!(store.model().soft_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note0.clone(), false);
        
        // Tuplize one note do nothing.
        store.tuplize(vec![Rc::new(note0.clone())]);
        store.wait_until_saved();
        assert_eq!(store.model().note_repo.len(), 1);
        assert_eq!(store.model().bar_repo.len(), 1); // bar replenished
        assert_eq!(store.model().note_repo.get(100u32), &vec![Rc::new(note0.clone())]);
        
        store.undo(); // this undo adding note.
        assert_eq!(store.model().note_repo.len(), 0);
        assert_eq!(store.model().bar_repo.len(), 0);
        
        let note1 = Note::new(
            110,
            Pitch::new(Solfa::D, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        
        let note2 = Note::new(
            120,
            Pitch::new(Solfa::E, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note0.clone(), false);
        store.add_note(note1.clone(), false);
        store.add_note(note2.clone(), false);
        
        store.tuplize(vec![Rc::new(note0.clone()), Rc::new(note1.clone()), Rc::new(note2.clone())]);
        store.wait_until_saved();
        
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        
        store.undo();
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().note_repo.get(100u32)[0].pitch.solfa(), Solfa::C);
        assert_eq!(store.model().note_repo.get(110u32)[0].pitch.solfa(), Solfa::D);
        assert_eq!(store.model().note_repo.get(120u32)[0].pitch.solfa(), Solfa::E);
        
        store.redo();
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        
        assert_eq!(store.model().note_repo.get(100u32)[0].pitch.solfa(), Solfa::C);
        assert_eq!(store.model().note_repo.get(180u32)[0].pitch.solfa(), Solfa::D);
        assert_eq!(store.model().note_repo.get(260u32)[0].pitch.solfa(), Solfa::E);
        
    }

    #[test]
    fn can_undo_bulk_remove() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        let tempo0 = Tempo::new(200, 200);
        store.bulk_add(
            Models {
                notes: vec![note0.clone()],
                bars: vec![], tempos: vec![tempo0], dumpers: vec![], softs: vec![]
            },
            ModelChangeMetadata::new()
        );
        store.wait_until_saved();
        assert_eq!(store.model().note_repo().len(), 1);
        assert_eq!(store.model().bar_repo().len(), 1); // bar is replenished.
        assert_eq!(store.model().tempo_repo().len(), 1);

        store.bulk_remove(
            Models {
                notes: vec![note0.clone()],
                bars: vec![], tempos: vec![tempo0], dumpers: vec![], softs: vec![]
            },
            ModelChangeMetadata::new()
        );
        store.wait_until_saved();

        assert_eq!(store.model().note_repo().len(), 0);
        assert_eq!(store.model().bar_repo().len(), 1);
        assert_eq!(store.model().tempo_repo().len(), 0);

        store.undo();

        assert_eq!(store.model().note_repo().len(), 1);
        assert_eq!(store.model().bar_repo().len(), 1); // bar is replenished.
        assert_eq!(store.model().tempo_repo().len(), 1);
    }

    #[test]
    fn can_undo_bulk_add() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );

        let tempo0 = Tempo::new(200, 200);
        store.bulk_add(
            Models {
                notes: vec![note0.clone()],
                bars: vec![], tempos: vec![tempo0], dumpers: vec![], softs: vec![]
            },
            ModelChangeMetadata::new()
        );
        store.wait_until_saved();

        assert_eq!(store.model().note_repo().len(), 1);
        assert_eq!(store.model().bar_repo().len(), 1); // bar is replenished.
        assert_eq!(store.model().tempo_repo().len(), 1);

        store.undo();

        assert_eq!(store.model().note_repo().len(), 0);
        assert_eq!(store.model().bar_repo().len(), 0);
        assert_eq!(store.model().tempo_repo().len(), 0);
    }

    #[test]
    fn can_undo_change() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new()).unwrap();
        
        let note00 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        let note01 = Note::new(
            101,
            Pitch::new(Solfa::C, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        let tempo0 = Tempo::new(200, 200);
        store.bulk_add(
            Models {
                notes: vec![note00.clone(), note01.clone()],
                bars: vec![], tempos: vec![tempo0], dumpers: vec![], softs: vec![]
            },
            ModelChangeMetadata::new()
        );
        store.wait_until_saved();

        assert_eq!(store.model().note_repo().len(), 2);
        assert_eq!(store.model().bar_repo().len(), 1); // bar is replenished.
        assert_eq!(store.model().tempo_repo().len(), 1);

        let note10 = note00.with_duration_numerator(Numerator::Half);
        let note11 = note01.with_duration_numerator(Numerator::Half);
        let change = ModelChanges {
            notes: vec![(note00.clone(), note10.clone()), (note01.clone(), note11.clone())],
            bars: vec![], tempos: vec![], dumpers: vec![], softs: vec![]
        };
        store.change(change, ModelChangeMetadata::new());
        store.wait_until_saved();

        assert_eq!(store.model().note_repo().len(), 2);
        let mut z = store.model().note_repo().iter();
        assert_eq!(z.next(), Some((&note10.start_tick(), &Rc::new(note10.clone()))));
        assert_eq!(z.next(), Some((&note11.start_tick(), &Rc::new(note11.clone()))));
        assert_eq!(z.next(), None);
        let mut z = store.model().tempo_repo().iter();
        assert_eq!(z.next(), Some(&(tempo0.start_tick, tempo0)));
        assert_eq!(z.next(), None);

        store.undo();

        assert_eq!(store.model().note_repo().len(), 2);
        let mut z = store.model().note_repo().iter();
        assert_eq!(z.next(), Some((&note00.start_tick(), &Rc::new(note00.clone()))));
        assert_eq!(z.next(), Some((&note01.start_tick(), &Rc::new(note01.clone()))));
        assert_eq!(z.next(), None);
        let mut z = store.model().tempo_repo().iter();
        assert_eq!(z.next(), Some(&(tempo0.start_tick, tempo0)));
        assert_eq!(z.next(), None);
    }

    #[test]
    fn many_changes() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new().with_undo_limit(10)).unwrap();
        
        let note00 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        let note01 = Note::new(
            101,
            Pitch::new(Solfa::C, Octave::Oct3, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO,
            Channel::default(),
        );
        store.add_note(note00, false);
        store.add_note(note01, false);

        for i in 0..20 {
            let tempo = Tempo::new(200 + i, 200 + i as u16);
            store.add_tempo(tempo, false);
        }
        assert_eq!(store.model().note_repo().len(), 2);
        assert_eq!(store.model().tempo_repo().len(), 20);
        drop(store);

        let store2 = SqliteUndoStore::<ProjectCmd, ProjectImpl, ProjectCmdErr>::open(dir.clone(), undo_store::Options::new().with_undo_limit(10)).unwrap();
        assert_eq!(store2.model().note_repo().len(), 2);
        assert_eq!(store2.model().tempo_repo().len(), 20);
    }
}
