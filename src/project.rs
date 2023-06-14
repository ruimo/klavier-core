use std::iter::zip;
use std::rc::Rc;

use klavier_helper::bag_store::{BagStore, BagStoreEvent};
use klavier_helper::store::{Store, StoreEvent};
use serde::{Serialize, Deserialize};
use serdo::undo_store::{SqliteUndoStore, UndoStore};
use serdo::cmd::{SerializableCmd, Cmd};

use crate::bar::{Bar, DcFine, EndOrRegion, RepeatStart};
use crate::ctrl_chg::CtrlChg;
use crate::grid::Grid;
use crate::key::Key;
use crate::location::Location;
use crate::models::{Models, ModelChanges};
use crate::note::Note;
use crate::rhythm::Rhythm;
use crate::tempo::{TempoValue, Tempo};
use crate::tuple;
use crate::undo::{Undo};
use crate::velocity::{Velocity, self};

const UNDO_LIMIT: usize = 100;
const DEFAULT_TEMPO: TempoValue = TempoValue::new(120);
const DEFAULT_CTRL_CHG: Velocity = velocity::MIN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationError {
    Overflow,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Project {
    rhythm: Rhythm,
    key: Key,
    grid: Grid,
    
    #[serde(skip)]
    undo_store: crate::undo::UndoStore,
    
    #[serde(skip)]
    #[serde(default = "new_note_repo")]
    note_repo: BagStore<u32, Rc<Note>, ModelChangeMetadata>, // by start tick.
    
    #[serde(skip)]
    #[serde(default = "new_bar_repo")]
    bar_repo: Store<u32, Bar, ModelChangeMetadata>,
    
    #[serde(skip)]
    #[serde(default = "new_tempo_repo")]
    tempo_repo: Store<u32, Tempo, ModelChangeMetadata>,
    
    #[serde(skip)]
    #[serde(default = "new_ctrlchg_repo")]
    dumper_repo: Store<u32, CtrlChg, ModelChangeMetadata>,
    
    #[serde(skip)]
    #[serde(default = "new_ctrlchg_repo")]
    soft_repo: Store<u32, CtrlChg, ModelChangeMetadata>,
}

fn new_note_repo() -> BagStore<u32, Rc<Note>, ModelChangeMetadata> {
    BagStore::new(false)
}

fn new_bar_repo() -> Store<u32, Bar, ModelChangeMetadata> {
    Store::new(false)
}

fn new_tempo_repo() -> Store<u32, Tempo, ModelChangeMetadata> {
    Store::new(false)
}

fn new_ctrlchg_repo() -> Store<u32, CtrlChg, ModelChangeMetadata> {
    Store::new(false)
}

impl Project {
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
    
    pub fn rhythm(&self) -> Rhythm {
        self.rhythm
    }
    
    pub fn set_rhythm(&mut self, rhythm: Rhythm) {
        self.rhythm = rhythm;
    }

    pub fn key(&self) -> Key {
        self.key
    }

    pub fn set_key(&mut self, key: Key) {
        self.key = key;
    }

    pub fn grid(&self) -> Grid {
        self.grid
    }

    pub fn set_grid(&mut self, grid: Grid) {
        self.grid = grid;
    }
    
    pub fn add_note(&mut self, note: Note, select: bool) {
        let note = Rc::new(note);
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true) }
        self.note_repo.add(note.start_tick(), note.clone(), metadata);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Changed {
                added: Models::empty().with_notes(&[note.clone()]).with_bars(replenishid_bars),
                removed: Models::empty(),
                metadata,
            }
        );
    }
    
    pub fn add_bar(&mut self, bar: Bar, select: bool) {
        let origin = self.add_bar_internal(bar, select);
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true) }
        self.undo_store.add(
            Undo::Changed {
                added: Models::empty().with_bars(vec![bar]),
                removed: Models::empty().with_bars(origin),
                metadata,
            }
        );
    }
    
    // Add bars without posting undo info.
    fn add_bar_internal(&mut self, bar: Bar, select: bool) -> Vec<Bar> {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        self.bar_repo.add(bar.start_tick, bar, metadata).map(|o| vec![o]).unwrap_or(vec![])
    }

    pub fn add_tempo(&mut self, tempo: Tempo, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let origin = self.tempo_repo.add(tempo.start_tick, tempo, metadata).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Changed {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![tempo],
                    dumpers: vec![],
                    softs: vec![],
                },
                removed: Models::empty().with_tempos(origin),
                metadata,
            }
        );
    }
    
    pub fn add_dumper(&mut self, ctrl_chg: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let origin = self.dumper_repo.add(ctrl_chg.start_tick, ctrl_chg, metadata).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Changed {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![],
                    dumpers: vec![ctrl_chg],
                    softs: vec![],
                },
                removed: Models::empty().with_dumpers(origin),
                metadata
            }
        );
    }
    
    pub fn add_soft(&mut self, ctrl_chg: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let origin = self.soft_repo.add(ctrl_chg.start_tick, ctrl_chg, metadata).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Changed {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![],
                    dumpers: vec![],
                    softs: vec![ctrl_chg],
                },
                removed: Models::empty().with_softs(origin),
                metadata,
            }
        );
    }
    
    pub fn tuplize(&mut self, notes: Vec<Rc<Note>>) {
        if 1 < notes.len() {
            let mut to_remove = Vec::with_capacity(notes.len());
            for n in notes.iter() {
                to_remove.push((n.start_tick(), n.clone()));
            }
            let tupled = tuple::tuplize(notes.clone());
            let to_add = tupled.iter().map(|n| (n.start_tick(), n.clone())).collect::<Vec<(u32, Rc<Note>)>>();
            let zipped = zip(to_remove.into_iter(), to_add.into_iter()).collect::<Vec<((u32, Rc<Note>), (u32, Rc<Note>))>>();

            self.note_repo.change(&zipped, ModelChangeMetadata::new().with_need_select(true));
            let replenishid_bars = self.replenish_bars();
            
            self.undo_store.add(
                Undo::Changed {
                    added: Models {
                        notes: Models::unwrap_rc(&tupled),
                        bars: replenishid_bars,
                        tempos: vec![],
                        dumpers: vec![],
                        softs: vec![],
                    },
                    removed: Models::empty().with_notes(&notes),
                    metadata: ModelChangeMetadata::new().with_need_select(true),
                }
            );
        }
    }
    
    pub fn bulk_remove(
        &mut self,
        notes: Vec<Rc<Note>>, bars: Vec<Bar>, tempoes: Vec<Tempo>, dumpers: Vec<CtrlChg>, softs: Vec<CtrlChg>,
        need_select: bool
    ) {
        let mut metadata = ModelChangeMetadata::new();
        if need_select { metadata.need_select = Some(true); }
        
        self.note_repo.bulk_remove(&notes.iter().map(|n| (n.start_tick(), n.clone())).collect::<Vec<(u32, Rc<Note>)>>(), metadata);
        self.bar_repo.bulk_remove(&bars.iter().map(|b| b.start_tick).collect::<Vec<u32>>(), metadata);
        self.tempo_repo.bulk_remove(&tempoes.iter().map(|t| t.start_tick).collect::<Vec<u32>>(), metadata);
        self.dumper_repo.bulk_remove(&dumpers.iter().map(|d| d.start_tick).collect::<Vec<u32>>(), metadata);
        self.soft_repo.bulk_remove(&softs.iter().map(|d| d.start_tick).collect::<Vec<u32>>(), metadata);

        let undo_remove = Models {
            notes: Models::unwrap_rc(&notes),
            bars,
            tempos: tempoes,
            dumpers,
            softs,
        };
        self.undo_store.add(Undo::Changed { added: Models::empty(), removed: undo_remove, metadata });
    }
    
    pub fn bulk_add(&mut self, mut to_add: Models, need_select: bool) {
        let mut removed = Models::empty();
        let mut metadata = ModelChangeMetadata::new();
        if need_select { metadata.need_select = Some(true); }

        let mut buf: Vec<(u32, Rc<Note>)> = Vec::with_capacity(to_add.notes.len());
        for n in to_add.notes.iter() {
            buf.push((n.start_tick(), Rc::new(n.clone())));
        }   
        self.note_repo.bulk_add(buf, metadata);

        let mut buf = Vec::with_capacity(to_add.bars.len());
        for b in to_add.bars.iter() {
            buf.push((b.start_tick, *b));
        }
        removed.bars = self.bar_repo.bulk_add(buf, metadata).iter().map(|(_, bar)| *bar).collect();

        let mut buf = Vec::with_capacity(to_add.tempos.len());
        for t in to_add.tempos.iter() {
            buf.push((t.start_tick, *t));
        }
        removed.tempos = self.tempo_repo.bulk_add(buf, metadata).iter().map(|(_, t)| *t).collect();


        let mut buf = Vec::with_capacity(to_add.dumpers.len());
        for d in to_add.dumpers.iter() {
            buf.push((d.start_tick, *d));
        }
        removed.dumpers = self.dumper_repo.bulk_add(buf, metadata).iter().map(|(_, d)| *d).collect();

        let mut buf = Vec::with_capacity(to_add.softs.len());
        for s in to_add.softs.iter() {
            buf.push((s.start_tick, *s));
        }
        removed.softs = self.soft_repo.bulk_add(buf, metadata).iter().map(|(_, s)| *s).collect();
        
        let replenished_bars = self.replenish_bars();
        to_add.bars.extend(replenished_bars);
        
        self.undo_store.add(Undo::Changed { added: to_add, removed, metadata });
    }
    
    pub fn change(&mut self, from_to: ModelChanges, metadata: ModelChangeMetadata) {
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
                (from.start_tick(), from.clone()), (to.start_tick(), to.clone())
            ));
            added.notes.push((**to).clone());
            removed.notes.push((**from).clone());
        }
        self.note_repo.change(&note_change, metadata);

        let mut bar_change: Vec<(&u32, (u32, Bar))> = Vec::with_capacity(from_to.bars.len());
        for (from, to) in from_to.bars.iter() {
            bar_change.push((
                &from.start_tick, (to.start_tick, *to)
            ));
            added.bars.push(*to);
            removed.bars.push(*from);
        }
        removed.bars.extend(self.bar_repo.change(&bar_change, metadata).iter().map(|(_, b)| *b).collect::<Vec<Bar>>());

        let mut tempo_change: Vec<(&u32, (u32, Tempo))> = Vec::with_capacity(from_to.tempos.len());
        for (from, to) in from_to.tempos.iter() {
            tempo_change.push((
                &from.start_tick, (to.start_tick, *to)
            ));
            added.tempos.push(*to);
            removed.tempos.push(*from);
        }
        removed.tempos.extend(self.tempo_repo.change(&tempo_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<Tempo>>());

        let mut dumper_change: Vec<(&u32, (u32, CtrlChg))> = Vec::with_capacity(from_to.dumpers.len());
        for (from, to) in from_to.dumpers.iter() {
            dumper_change.push((
                &from.start_tick, (to.start_tick, *to)
            ));
            added.dumpers.push(*to);
            removed.dumpers.push(*from);
        }
        removed.dumpers.extend(self.dumper_repo.change(&dumper_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<CtrlChg>>());

        let mut soft_change: Vec<(&u32, (u32, CtrlChg))> = Vec::with_capacity(from_to.softs.len());
        for (from, to) in from_to.softs.iter() {
            soft_change.push((
                &from.start_tick, (to.start_tick, *to)
            ));
            added.softs.push(*to);
            removed.softs.push(*from);
        }
        removed.softs.extend(self.soft_repo.change(&soft_change,metadata).iter().map(|(_, t)| *t).collect::<Vec<CtrlChg>>());

        added.bars.extend(self.replenish_bars());
        self.undo_store.add(Undo::Changed { added, removed, metadata });
    }
    
    pub fn can_undo(&self) -> bool {
        self.undo_store.can_undo()
    }
    
    pub fn undo(&mut self) {
        self.undo_store.freeze(true);
        if let Some(u) = self.undo_store.undo() {
            match u {
                Undo::Changed { added, removed, metadata } => {
                    for n in added.notes.iter() {
                        self.note_repo.remove(&n.start_tick(), &Rc::new(n.clone()));
                    }
                    for b in added.bars.iter() {
                        self.bar_repo.remove(&b.start_tick);
                    }
                    for t in added.tempos.iter() {
                        self.tempo_repo.remove(&t.start_tick);
                    }
                    for d in added.dumpers.iter() {
                        self.dumper_repo.remove(&d.start_tick);
                    }
                    for s in added.softs.iter() {
                        self.soft_repo.remove(&&s.start_tick);
                    }
                    
                    for n in removed.notes.iter() {
                        self.note_repo.add(n.start_tick(), Rc::new(n.clone()), *metadata);
                    }
                    for b in removed.bars.iter() {
                        self.bar_repo.add(b.start_tick, *b, *metadata);
                    }
                    for t in removed.tempos.iter() {
                        self.tempo_repo.add(t.start_tick, *t, *metadata);
                    }
                    for d in removed.dumpers.iter() {
                        self.dumper_repo.add(d.start_tick, *d, *metadata);
                    }
                    for s in removed.softs.iter() {
                        self.soft_repo.add(s.start_tick, *s, *metadata);
                    }
                },
            }
        }
        self.undo_store.freeze(false);
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
    
    pub fn clear_model_events(&mut self) {
        self.note_repo.clear_events();
        self.bar_repo.clear_events();
        self.tempo_repo.clear_events();
        self.dumper_repo.clear_events();
        self.soft_repo.clear_events();
    }
    
    pub fn bar_events(&self) -> &Vec<StoreEvent<u32, Bar, ModelChangeMetadata>> {
        self.bar_repo.events()
    }
    
    pub fn tempo_events(&self) -> &Vec<StoreEvent<u32, Tempo, ModelChangeMetadata>> {
        self.tempo_repo.events()
    }
    
    pub fn dumper_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>> {
        self.dumper_repo.events()
    }
    
    pub fn soft_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, ModelChangeMetadata>> {
        self.soft_repo.events()
    }
    
    pub fn note_events(&self) -> &Vec<BagStoreEvent<u32, Rc<Note>, ModelChangeMetadata>> {
        self.note_repo.events()
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
                Err(LocationError::Overflow)
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
    
    #[inline]
    fn last_bar(&self) -> Option<Bar> {
        self.bar_repo.peek_last().map(|(_, bar)| bar.clone())
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
        let mut bar_tick = self.last_bar().map(|b| b.start_tick).unwrap_or(0);
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
            let bar = Bar::new(bar_tick, None, None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null);
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

impl Default for Project {
    fn default() -> Self {
        Project {
            rhythm: Rhythm::default(),
            key: Key::NONE,
            grid: Grid::default(),
            undo_store: crate::undo::UndoStore::new(UNDO_LIMIT),
            note_repo: BagStore::new(true),
            bar_repo: Store::new(true),
            tempo_repo: Store::new(true),
            dumper_repo: Store::new(true),
            soft_repo: Store::new(true),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ProjectCmd {
    SetRhythm(Rhythm, Rhythm),
    SetKey(Key, Key),
    SetGrid(Grid, Grid),
    ModelChanged { added: Models, removed: Models, metadata: ModelChangeMetadata },
}

impl Cmd for ProjectCmd {
    type Model = Project;
    
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

enum ProjectCmdErr {
    NoOp,
}

trait ProjectIntf {
    fn set_rhythm(&mut self, rhythm: Rhythm);
    fn set_key(&mut self, key: Key);
    fn set_grid(&mut self, key: Grid);
    fn add_note(&mut self, note: Note, select: bool);
    fn add_bar(&mut self, bar: Bar, select: bool);
    fn add_tempo(&mut self, bar: Tempo, select: bool);
    fn add_dumper(&mut self, dumper: CtrlChg, select: bool);
    fn add_soft(&mut self, soft: CtrlChg, select: bool);
    fn tuplize(&mut self, notes: Vec<Rc<Note>>);
    fn bulk_remove(
        &mut self,
        note_remove: Vec<Rc<Note>>, bar_remove: Vec<Bar>, tempo_remove: Vec<Tempo>, dumper_remove: Vec<CtrlChg>, soft_remove: Vec<CtrlChg>,
        need_select: bool
    );
}

impl ProjectIntf for SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr> {
    fn set_rhythm(&mut self, rhythm: Rhythm) {
        self.add_cmd(ProjectCmd::SetRhythm(self.model().rhythm, rhythm));
    }
    
    fn set_key(&mut self, key: Key) {
        self.add_cmd(ProjectCmd::SetKey(self.model().key, key));
    }
    
    fn set_grid(&mut self, grid: Grid) {
        self.add_cmd(ProjectCmd::SetGrid(self.model().grid, grid));
    }
    
    fn add_note(&mut self, note: Note, select: bool) {
        let note = Rc::new(note);
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }

        let _ = self.mutate(&mut |proj| {
            proj.note_repo.add(note.start_tick(), note.clone(), metadata);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_notes(&[note.clone()]).with_bars(replenishid_bars),
                    removed: Models::empty(),
                    metadata,
                }
            )
        });
    }
    
    fn add_bar(&mut self, bar: Bar, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(&mut | proj| {
            let origin = proj.bar_repo.add(bar.start_tick, bar, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(vec![bar]),
                    removed: Models::empty().with_bars(origin),
                    metadata,
                }
            )
        });
    }
    
    fn add_tempo(&mut self, tempo: Tempo, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(&mut |proj| {
            let origin = proj.tempo_repo.add(tempo.start_tick, tempo, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_tempos(vec![tempo]),
                    removed: Models::empty().with_tempos(origin),
                    metadata,
                }
                
            )
        });
    }
    
    fn add_dumper(&mut self, dumper: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(&mut |proj| {
            let origin = proj.dumper_repo.add(dumper.start_tick, dumper, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_dumpers(vec![dumper]),
                    removed: Models::empty().with_dumpers(origin),
                    metadata,
                }
            )
        });
    }
    
    fn add_soft(&mut self, soft: CtrlChg, select: bool) {
        let mut metadata = ModelChangeMetadata::new();
        if select { metadata.need_select = Some(true); }
        let _ = self.mutate(&mut |proj| {
            let origin = proj.soft_repo.add(soft.start_tick, soft, metadata).map(|o| vec![o]).unwrap_or(vec![]);
            let replenishid_bars = proj.replenish_bars();
            Ok(
                ProjectCmd::ModelChanged {
                    added: Models::empty().with_bars(replenishid_bars).with_softs(vec![soft]),
                    removed: Models::empty().with_softs(origin),
                    metadata
                }
            )
        });
    }
    
    fn tuplize(&mut self, notes: Vec<Rc<Note>>) {
        let metadata = ModelChangeMetadata::new().with_need_select(true);
        let _ = self.mutate(&mut |proj| {
            if 1 < notes.len() {
                let mut to_remove = Vec::with_capacity(notes.len());
                for n in notes.iter() {
                    to_remove.push((n.start_tick(), n.clone()));
                }
                let tupled = tuple::tuplize(notes.clone());
                proj.note_repo.remove_all(&to_remove);
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
                Err(ProjectCmdErr::NoOp)
            }
        });
        
    }

    fn bulk_remove(
        &mut self,
        note_remove: Vec<Rc<Note>>, bar_remove: Vec<Bar>, tempo_remove: Vec<Tempo>, dumper_remove: Vec<CtrlChg>, soft_remove: Vec<CtrlChg>,
        need_select: bool
    ) {
        let mut metadata = ModelChangeMetadata::new();
        if need_select { metadata.need_select = Some(true); }
        let removed = Models {
            notes: Models::unwrap_rc(&note_remove),
            bars: bar_remove,
            tempos: tempo_remove,
            dumpers: dumper_remove,
            softs: soft_remove,
        };

        self.add_cmd(
            ProjectCmd::ModelChanged { added: Models::empty(), removed, metadata }
        );
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    
    use klavier_helper::store::Store;
    use serdo::undo_store::{SqliteUndoStore, UndoStore};
    
    use crate::{tempo::{Tempo, TempoValue}, project::{tempo_at, LocationError, ProjectCmd, ProjectCmdErr, ModelChangeMetadata}, note::Note, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, pitch::Pitch, duration::{Duration, Numerator, Denominator, Dots}, velocity::Velocity, trimmer::{Trimmer, RateTrimmer}, bar::{Bar, DcFine, EndOrRegion, RepeatStart}, location::Location, rhythm::Rhythm, ctrl_chg::CtrlChg, key::Key, grid::Grid};
    
    use super::{DEFAULT_TEMPO, Project};
    
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
        let mut proj = Project::default();
        let note = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false,
            false,
            Velocity::new(64),
            Trimmer::ZERO,
            RateTrimmer::ONE,
            Trimmer::ZERO
        );
        
        let _ = proj.add_note(note, false);
        assert_eq!(proj.note_repo().len(), 1);
        
        proj.undo();
        assert_eq!(proj.note_repo().len(), 0);
    }
    
    #[test]
    fn undo_bar_addition() {
        let mut proj = Project::default();
        let bar = Bar::new(
            100,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        proj.add_bar(bar, false);
        assert_eq!(proj.bar_repo().len(), 1);
        proj.undo();
        assert_eq!(proj.bar_repo().len(), 0);
    }
    
    #[test]
    fn location_to_tick() {
        let mut proj = Project::default();
        assert_eq!(proj.location_to_tick(Location::new(0, 0)), Ok(0));
        assert_eq!(proj.location_to_tick(Location::new(0, 1)), Ok(1));
        assert_eq!(proj.location_to_tick(Location::new(0, u32::MAX as usize)), Ok(u32::MAX));
        assert_eq!(proj.location_to_tick(Location::new(0, u32::MAX as usize + 1)), Err(LocationError::Overflow));
        assert_eq!(proj.location_to_tick(Location::new(1, 1)), Err(LocationError::Overflow));
        
        let bar = Bar::new(
            100,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        proj.add_bar(bar, false);
        
        assert_eq!(proj.location_to_tick(Location::new(0, 0)), Ok(0));
        assert_eq!(proj.location_to_tick(Location::new(0, 1)), Ok(1));
        assert_eq!(proj.location_to_tick(Location::new(1, 1)), Ok(101));
        assert_eq!(proj.location_to_tick(Location::new(1, (u32::MAX as usize) - 100)), Ok(u32::MAX));
        assert_eq!(proj.location_to_tick(Location::new(1, (u32::MAX as usize) - 99)), Err(LocationError::Overflow));
        assert_eq!(proj.location_to_tick(Location::new(2, 0)), Err(LocationError::Overflow));
        
        let bar = Bar::new(
            1000,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        proj.add_bar(bar, false);
        
        assert_eq!(proj.location_to_tick(Location::new(0, 0)), Ok(0));
        assert_eq!(proj.location_to_tick(Location::new(0, 1)), Ok(1));
        assert_eq!(proj.location_to_tick(Location::new(1, 1)), Ok(101));
        assert_eq!(proj.location_to_tick(Location::new(2, 0)), Ok(1000));
    }
    
    #[test]
    fn tick_to_location() {
        let mut proj = Project::default();
        assert_eq!(proj.tick_to_location(0), Location::new(0, 0));
        assert_eq!(proj.tick_to_location(100), Location::new(0, 100));
        assert_eq!(proj.tick_to_location(u32::MAX), Location::new(0, u32::MAX as usize));
        
        let bar = Bar::new(
            100,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        proj.add_bar(bar, false);
        
        assert_eq!(proj.tick_to_location(0), Location::new(0, 0));
        assert_eq!(proj.tick_to_location(99), Location::new(0, 99));
        assert_eq!(proj.tick_to_location(100), Location::new(1, 0));
        assert_eq!(proj.tick_to_location(u32::MAX), Location::new(1, (u32::MAX - 100) as usize));
        
        let bar = Bar::new(
            1000,
            None,
            None,
            DcFine::Null,
            EndOrRegion::Null,
            RepeatStart::Null
        );
        proj.add_bar(bar, false);
        
        assert_eq!(proj.tick_to_location(0), Location::new(0, 0));
        assert_eq!(proj.tick_to_location(99), Location::new(0, 99));
        assert_eq!(proj.tick_to_location(999), Location::new(1, 899));
        assert_eq!(proj.tick_to_location(1000), Location::new(2, 0));
        assert_eq!(proj.tick_to_location(u32::MAX), Location::new(2, (u32::MAX - 1000) as usize));
    }
    
    #[test]
    fn rhythm_at() {
        let mut proj = Project::default();
        proj.rhythm = Rhythm::new(6, 8);
        assert_eq!(proj.last_bar(), None);
        
        assert_eq!(proj.rhythm_at(500), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(0), Rhythm::new(6, 8));
        
        let bar0 = Bar::new(
            100, None,
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar0, false);
        assert_eq!(proj.last_bar(), Some(bar0));
        
        assert_eq!(proj.rhythm_at(0), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(99), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(100), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(101), Rhythm::new(6, 8));
        
        let bar1 = Bar::new(
            200, Some(Rhythm::new(3, 4)),
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar1, false);
        assert_eq!(proj.last_bar(), Some(bar1));
        
        let bar2 = Bar::new(
            300, None,
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar2, false);
        assert_eq!(proj.last_bar(), Some(bar2));
        
        let bar3 = Bar::new(
            400, Some(Rhythm::new(4, 4)),
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar3, false);
        assert_eq!(proj.last_bar(), Some(bar3));
        
        assert_eq!(proj.rhythm_at(0), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(99), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(100), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(101), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(199), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(200), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(201), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(299), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(300), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(301), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(399), Rhythm::new(3, 4));
        assert_eq!(proj.rhythm_at(400), Rhythm::new(4, 4));
        assert_eq!(proj.rhythm_at(401), Rhythm::new(4, 4));
    }
    
    #[test]
    fn note_max_tick_loc() {
        let mut proj = Project::default();
        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        assert_eq!(proj.note_max_end_tick(), None);
        
        let note0 = Note::new( // end tick: 100 + 240 * 1.5 = 460
            100, pitch,
            Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO
        );
        
        let end_tick0 = note0.base_start_tick() + note0.tick_len();
        proj.add_note(note0, false);
        assert_eq!(proj.note_max_end_tick(), Some(end_tick0));
        
        let note1 = Note::new( // end tick: 100 + 960 * 1.5 = 1540
            100, pitch,
            Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ONE),
            false, false, Velocity::new(10), Trimmer::ZERO,
            RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
            Trimmer::ZERO
        );
        
        proj.add_note(note1.clone(), false);
        assert_eq!(proj.note_max_end_tick(), Some(note1.base_start_tick() + note1.tick_len()));
        
        let _ = proj.add_note(
            Note::new( // end tick: 200 + 120 * 2.5 = 440
                200, pitch,
                Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 2.0),
                Trimmer::ZERO
            ), false
        );
        
        assert_eq!(proj.note_max_end_tick(), Some(note1.base_start_tick() + note1.tick_len()));
    }
    
    #[test]
    fn replenish_bars() {
        let mut proj = Project::default(); // Default rhythm = 4/4 = 960tick/bar
        let pitch = Pitch::new(Solfa::C, Octave::Oct0, SharpFlat::Null);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 0);
        let _ = proj.add_note(
            Note::new( // end tick: 100 + 240 * 1.5 = 460
                100, pitch,
                Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO
            ), false
        );
        assert_eq!(proj.bar_repo().len(), 1);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 1);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960);
        
        let _ = proj.add_note(
            Note::new(
                0, pitch,
                Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO
            ), false
        );
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 1);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960);
        
        let _ = proj.add_note(
            Note::new(
                960, pitch,
                Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO
            ), false
        );
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 3);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 3);
        
        let _ = proj.add_tempo(Tempo::new(960*3, 200), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 3);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 3);
        
        let _ = proj.add_tempo(Tempo::new(960 * 3 + 1, 200), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 4);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 4);
        
        let _ = proj.add_dumper(CtrlChg::new(960 * 4, Velocity::new(20)), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 4);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 4);
        
        let _ = proj.add_dumper(CtrlChg::new(960 * 4 + 1, Velocity::new(20)), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 5);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 5);
        
        let _ = proj.add_soft(CtrlChg::new(960 * 5, Velocity::new(20)), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 5);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 5);
        
        let _ = proj.add_dumper(CtrlChg::new(960 * 5 + 1, Velocity::new(20)), false);
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 6);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 6);
    }
    
    #[test]
    fn tuplize() {
        let mut proj = Project::default();
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        proj.add_note(note0.clone(), false);
        
        let note1 = Note::new(
            120,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        proj.add_note(note1.clone(), false);
        
        let note2 = Note::new(
            150,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        proj.add_note(note2.clone(), false);
        assert_eq!(proj.note_repo().len(), 3);
        
        proj.tuplize(vec![Rc::new(note0), Rc::new(note1), Rc::new(note2)]);
        assert_eq!(proj.note_repo().len(), 3);
        
        let mut z = proj.note_repo().iter();
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100 + 80);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100 + 80 * 2);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(3).unwrap(), Dots::ZERO));
        
        proj.undo();
        assert_eq!(proj.note_repo().len(), 3);
        
        let mut z = proj.note_repo().iter();
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 100);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 120);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
        
        let (tick, note) = z.next().unwrap();
        assert_eq!(*tick, 150);
        assert_eq!(note.duration(), Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO));
    }
    
    #[test]
    fn can_serialize_project() {
        let mut proj = Project::default();
        proj.key = Key::FLAT_2;
        proj.rhythm = Rhythm::new(3, 4);
        
        let ser = bincode::serialize(&proj).unwrap();
        
        let des: Project = bincode::deserialize(&ser).unwrap();
        
        assert_eq!(proj.key, des.key);
        assert_eq!(proj.rhythm, des.rhythm);
    }
    
    use tempfile::tempdir;
    use super::ProjectIntf;
    
    #[test]
    fn can_undo_set_rhythm() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        store.set_rhythm(Rhythm::new(12, 8));
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.set_rhythm(Rhythm::new(12, 4));
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 4));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::default());
        
        store.redo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
        
        store.set_rhythm(Rhythm::new(16, 8));
        assert_eq!(store.model().rhythm(), Rhythm::new(16, 8));
        
        store.undo();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
    }
    
    #[test]
    fn can_undo_set_key() {
        let mut dir = tempdir().unwrap().as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        store.set_key(Key::FLAT_1);
        store.set_key(Key::FLAT_2);
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        store.set_grid(Grid::from_u32(100).unwrap());
        store.set_grid(Grid::from_u32(200).unwrap());
        
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        let note0 = Note::new(
            100,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        
        store.add_note(note0.clone(), false);
        
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        let bar0 = Bar::new(
            1000, Some(Rhythm::new(3, 4)),
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        
        store.add_bar(bar0, false);
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        let tempo0 = Tempo::new(200, 200);
        
        store.add_tempo(tempo0, false);
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        let dumper = CtrlChg::new(200, Velocity::new(20));
        store.add_dumper(dumper, false);
        
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
        let soft = CtrlChg::new(200, Velocity::new(20));
        store.add_soft(soft, false);
        
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
        let mut store = SqliteUndoStore::<ProjectCmd, Project, ProjectCmdErr>::open(dir.clone(), None).unwrap();
        
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
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        store.add_note(note0.clone(), false);
        
        // Tuplize one note do nothing.
        store.tuplize(vec![Rc::new(note0.clone())]);
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
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        
        let note2 = Note::new(
            120,
            Pitch::new(Solfa::E, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        store.add_note(note0.clone(), false);
        store.add_note(note1.clone(), false);
        store.add_note(note2.clone(), false);
        
        store.tuplize(vec![Rc::new(note0.clone()), Rc::new(note1.clone()), Rc::new(note2.clone())]);
        
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        
        store.undo();
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        assert_eq!(store.model().note_repo.get(100u32)[0].pitch().solfa(), Solfa::C);
        assert_eq!(store.model().note_repo.get(110u32)[0].pitch().solfa(), Solfa::D);
        assert_eq!(store.model().note_repo.get(120u32)[0].pitch().solfa(), Solfa::E);
        
        store.redo();
        assert_eq!(store.model().note_repo.len(), 3);
        assert_eq!(store.model().bar_repo.len(), 1);
        
        assert_eq!(store.model().note_repo.get(100u32)[0].pitch().solfa(), Solfa::C);
        assert_eq!(store.model().note_repo.get(180u32)[0].pitch().solfa(), Solfa::D);
        assert_eq!(store.model().note_repo.get(260u32)[0].pitch().solfa(), Solfa::E);
        
    }

    #[test]
    fn can_undo_changes() {
        
    }
}