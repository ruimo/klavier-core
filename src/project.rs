use std::collections::BTreeMap;
use std::rc::Rc;

use klavier_helper::bag_store::{BagStore, BagStoreEvent};
use klavier_helper::{bulk_remove, changes};
use klavier_helper::store::{Store, StoreEvent};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use serde::ser::SerializeStruct;
use serdo::undo_store::{SqliteUndoStore, SqliteUndoStoreAddCmdError, UndoStore};
use serdo::cmd::{SerializableCmd, Cmd};

use crate::bar::{Bar, DcFine, EndOrRegion, RepeatStart};
use crate::ctrl_chg::CtrlChg;
use crate::grid::Grid;
use crate::key::Key;
use crate::location::Location;
use crate::models::Models;
use crate::note::Note;
use crate::rhythm::Rhythm;
use crate::tempo::{TempoValue, Tempo};
use crate::tuple;
use crate::undo::{Undo, ModelChanges};
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Project {
    rhythm: Rhythm,
    key: Key,

    #[serde(skip)]
    grid: Grid,

    #[serde(skip)]
    undo_store: crate::undo::UndoStore,

    #[serde(skip)]
    #[serde(default = "new_note_repo")]
    note_repo: BagStore<u32, Rc<Note>, Option<bool>>, // by start tick.

    #[serde(skip)]
    #[serde(default = "new_bar_repo")]
    bar_repo: Store<u32, Bar, Option<bool>>,

    #[serde(skip)]
    #[serde(default = "new_tempo_repo")]
    tempo_repo: Store<u32, Tempo, Option<bool>>,

    #[serde(skip)]
    #[serde(default = "new_ctrlchg_repo")]
    dumper_repo: Store<u32, CtrlChg, Option<bool>>,

    #[serde(skip)]
    #[serde(default = "new_ctrlchg_repo")]
    soft_repo: Store<u32, CtrlChg, Option<bool>>,
}

fn new_note_repo() -> BagStore<u32, Rc<Note>, Option<bool>> {
    BagStore::new(false)
}

fn new_bar_repo() -> Store<u32, Bar, Option<bool>> {
    Store::new(false)
}

fn new_tempo_repo() -> Store<u32, Tempo, Option<bool>> {
    Store::new(false)
}

fn new_ctrlchg_repo() -> Store<u32, CtrlChg, Option<bool>> {
    Store::new(false)
}

impl Project {
    pub fn note_repo(&self) -> &BagStore<u32, Rc<Note>, Option<bool>> {
        &self.note_repo
    }

    pub fn bar_repo(&self) -> &Store<u32, Bar, Option<bool>> {
        &self.bar_repo
    }

    pub fn tempo_repo(&self) -> &Store<u32, Tempo, Option<bool>> {
        &self.tempo_repo
    }

    pub fn dumper_repo(&self) -> &Store<u32, CtrlChg, Option<bool>> {
        &self.dumper_repo
    }

    pub fn rhythm(&self) -> Rhythm {
        self.rhythm
    }

//    pub fn set_rhythm(&mut self, rhythm: Rhythm) {
//        self.rhythm = rhythm;
//    }

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

    pub fn add_note(&mut self, note: Note) -> Rc<Note> {
        let note = Rc::new(note);
        self.note_repo.add(note.start_tick(), note.clone());
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Added {
                added: Models {
                    notes: vec![note.clone()],
                    bars: replenishid_bars,
                    tempos: vec![],
                    dumpers: vec![],
                    softs: vec![],
                },
                removed: Models::empty(),
            }
        );
        note
    }

    pub fn add_bar(&mut self, bar: Bar) {
        let origin = self.bar_repo.add(bar.start_tick, bar).map(|o| vec![o]).unwrap_or(vec![]);
        self.undo_store.add(
            Undo::Added {
                added: Models::bar_only(vec![bar]),
                removed: Models::bar_only(origin),
            }
        );
    }

    pub fn add_tempo(&mut self, tempo: Tempo) {
        let origin = self.tempo_repo.add(tempo.start_tick, tempo).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Added {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![tempo],
                    dumpers: vec![],
                    softs: vec![],
                },
                removed: Models::tempo_only(origin),
            }
        );
    }

    pub fn add_dumper(&mut self, ctrl_chg: CtrlChg) {
        let origin = self.dumper_repo.add(ctrl_chg.start_tick, ctrl_chg).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Added {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![],
                    dumpers: vec![ctrl_chg],
                    softs: vec![],
                },
                removed: Models::dumper_only(origin)
            }
        );
    }

    pub fn add_soft(&mut self, ctrl_chg: CtrlChg) {
        let origin = self.soft_repo.add(ctrl_chg.start_tick, ctrl_chg).map(|o| vec![o]).unwrap_or(vec![]);
        let replenishid_bars = self.replenish_bars();
        self.undo_store.add(
            Undo::Added {
                added: Models {
                    notes: vec![],
                    bars: replenishid_bars,
                    tempos: vec![],
                    dumpers: vec![],
                    softs: vec![ctrl_chg],
                },
                removed: Models::soft_only(origin),
            }
        );
    }

    pub fn tuplize(&mut self, notes: Vec<Rc<Note>>) {
        if 1 < notes.len() {
            let mut to_remove = BTreeMap::new();
            for n in notes.iter() {
                to_remove.insert(n.start_tick(), vec![n.clone()]);
            }
            let tupled = tuple::tuplize(notes.clone());
            self.note_repo.remove_all(to_remove);
            self.note_repo.bulk_add(
                tupled.iter().map(|n| (n.start_tick(), n.clone())).collect(),
                Some(true)
            );
            let replenishid_bars = self.replenish_bars();

            self.undo_store.add(
                Undo::Added {
                    added: Models {
                        notes: tupled,
                        bars: replenishid_bars,
                        tempos: vec![],
                        dumpers: vec![],
                        softs: vec![],
                    },
                    removed: Models::note_only(notes)
                }
            );
        }
    }

    pub fn bulk_remove(
        &mut self,
        note_remove: Option<impl bulk_remove::BulkRemove<u32, Rc<Note>> + 'static>,
        bar_remove: Option<impl bulk_remove::BulkRemove<u32, Bar> + 'static>,
        tempo_remove: Option<impl bulk_remove::BulkRemove<u32, Tempo> + 'static>,
        dumper_remove: Option<impl bulk_remove::BulkRemove<u32, CtrlChg> + 'static>,
        soft_remove: Option<impl bulk_remove::BulkRemove<u32, CtrlChg> + 'static>
    ) {
        let mut undo_remove = Models {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        };

        if let Some(nr) = note_remove {
            for (_k, note) in nr.iter() {
                undo_remove.notes.push(note);
            }
            self.note_repo.bulk_remove(nr);
        }
        if let Some(br) = bar_remove {
            for (_k, v) in br.iter() {
                undo_remove.bars.push(v);
            }
            self.bar_repo.bulk_remove(br);
        }
        if let Some(tr) = tempo_remove {
            for (_k, v) in tr.iter() {
                undo_remove.tempos.push(v);
            }
            self.tempo_repo.bulk_remove(tr);
        }
        if let Some(dr) = dumper_remove {
            for (_k, v) in dr.iter() {
                undo_remove.dumpers.push(v);
            }
            self.dumper_repo.bulk_remove(dr);
        }
        if let Some(sr) = soft_remove {
            for (_k, v) in sr.iter() {
                undo_remove.softs.push(v);
            }
            self.soft_repo.bulk_remove(sr);
        }

        self.undo_store.add(
            Undo::Removed(undo_remove)
        );
    }

    pub fn copy_with(
        &mut self,
        note_changes: Option<impl changes::Changes<u32, Rc<Note>> + 'static>,
        bar_changes: Option<impl changes::Changes<u32, Bar> + 'static>,
        tempo_changes: Option<impl changes::Changes<u32, Tempo> + 'static>,
        dumper_changes: Option<impl changes::Changes<u32, CtrlChg> + 'static>,
        soft_changes: Option<impl changes::Changes<u32, CtrlChg> + 'static>
    ) {
        let mut undo_rec = Models {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        };

        let mut undo_removed_rec = Models {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        };

        if let Some(note_changes) = note_changes {
            let mut buf = Vec::with_capacity(note_changes.len());
            for (_, (to_key, to)) in note_changes.iter() {
                undo_rec.notes.push(to.clone());
                buf.push((to_key.clone(), to.clone()));
            }   
            self.note_repo.bulk_add(buf, Some(true));
        }
        if let Some(bar_changes) = bar_changes {
            let mut buf = Vec::with_capacity(bar_changes.len());
            for (_, (to_key, to)) in bar_changes.iter() {
                undo_rec.bars.push(to.clone());
                buf.push((to_key.clone(), to.clone()));
            }
            undo_removed_rec.bars = self.bar_repo.bulk_add(buf, Some(true)).iter().map(|(_, bar)| *bar).collect();
        }
        if let Some(tempo_changes) = tempo_changes {
            let mut buf = Vec::with_capacity(tempo_changes.len());
            for (_, (to_key, to)) in tempo_changes.iter() {
                undo_rec.tempos.push(to.clone());
                buf.push((to_key.clone(), to.clone()));
            }
            undo_removed_rec.tempos = self.tempo_repo.bulk_add(buf, Some(true)).iter().map(|(_, t)| *t).collect();
        }
        if let Some(dumper_changes) = dumper_changes {
            let mut buf = Vec::with_capacity(dumper_changes.len());
            for (_, (to_key, to)) in dumper_changes.iter() {
                undo_rec.dumpers.push(to.clone());
                buf.push((to_key.clone(), to.clone()));
            }
            undo_removed_rec.dumpers = self.dumper_repo.bulk_add(buf, Some(true)).iter().map(|(_, d)| *d).collect();
        }
        if let Some(soft_changes) = soft_changes {
            let mut buf = Vec::with_capacity(soft_changes.len());
            for (_, (to_key, to)) in soft_changes.iter() {
                undo_rec.softs.push(to.clone());
                buf.push((to_key.clone(), to.clone()));
            }
            undo_removed_rec.softs = self.soft_repo.bulk_add(buf, Some(true)).iter().map(|(_, s)| *s).collect();
        }

        let replenished_bars = self.replenish_bars();
        undo_rec.bars.extend(replenished_bars);

        self.undo_store.add(Undo::Added { added: undo_rec, removed: undo_removed_rec });
    }

    pub fn change_repo(
        &mut self,
        note_changes: Option<impl changes::Changes<u32, Rc<Note>> + 'static>,
        bar_changes: Option<impl changes::Changes<u32, Bar> + 'static>,
        tempo_changes: Option<impl changes::Changes<u32, Tempo> + 'static>,
        dumper_changes: Option<impl changes::Changes<u32, CtrlChg> + 'static>,
        soft_changes: Option<impl changes::Changes<u32, CtrlChg> + 'static>
    ) {
        let mut undo_changes = ModelChanges {
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![],
        };

        let mut undo_removed = Models::empty();
        if let Some(note_changes) = note_changes {
            for ((_, from), (_, to)) in note_changes.iter() {
                undo_changes.notes.push((from.clone(), to.clone()));
            }   
            self.note_repo.change(note_changes);
        }
        if let Some(bar_changes) = bar_changes {
            for ((_, from), (_, to)) in bar_changes.iter() {
                undo_changes.bars.push((from.clone(), to.clone()));
            }
            undo_removed.bars = self.bar_repo.change(bar_changes).iter().map(|(_, b)| *b).collect();
        }
        if let Some(tempo_changes) = tempo_changes {
            for ((_, from), (_, to)) in tempo_changes.iter() {
                undo_changes.tempos.push((from.clone(), to.clone()));
            }
            undo_removed.tempos = self.tempo_repo.change(tempo_changes).iter().map(|(_, t)| *t).collect();
        }
        if let Some(dumper_changes) = dumper_changes {
            for ((_, from), (_, to)) in dumper_changes.iter() {
                undo_changes.dumpers.push((from.clone(), to.clone()));
            }
            undo_removed.dumpers = self.dumper_repo.change(dumper_changes).iter().map(|(_, d)| *d).collect();
        }
        if let Some(soft_changes) = soft_changes {
            for ((_, from), (_, to)) in soft_changes.iter() {
                undo_changes.softs.push((from.clone(), to.clone()));
            }
            undo_removed.softs = self.soft_repo.change(soft_changes).iter().map(|(_, s)| *s).collect();
        }

        self.undo_store.add(Undo::Changed { changed: undo_changes, removed: undo_removed });

        let replenished_bars = self.replenish_bars();
        if ! replenished_bars.is_empty() {
            self.undo_store.add(
                Undo::Added {
                    added: Models::bar_only(replenished_bars),
                    removed: Models::empty(),
                }
            );
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo_store.can_undo()
    }

    pub fn undo(&mut self) {
        self.undo_store.freeze(true);
        if let Some(u) = self.undo_store.undo() {
            match u {
                Undo::Added { added, removed } => {
                    for n in added.notes.iter() {
                        self.note_repo.remove(&n.start_tick(), n);
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
                        self.note_repo.add(n.start_tick(), n.clone());
                    }
                    for b in removed.bars.iter() {
                        self.bar_repo.add(b.start_tick, *b);
                    }
                    for t in removed.tempos.iter() {
                        self.tempo_repo.add(t.start_tick, *t);
                    }
                    for d in removed.dumpers.iter() {
                        self.dumper_repo.add(d.start_tick, *d);
                    }
                    for s in removed.softs.iter() {
                        self.soft_repo.add(s.start_tick, *s);
                    }
                },
                Undo::Changed { changed, removed } => {
                    for (from, to) in changed.notes.iter() {
                        self.note_repo.remove(&to.start_tick(), to);
                        self.note_repo.add(from.start_tick(), from.clone());
                    }
                    for (from, to) in changed.bars.iter() {
                        self.bar_repo.remove(&to.start_tick);
                        self.bar_repo.add(from.start_tick, from.clone());
                    }
                    for (from, to) in changed.tempos.iter() {
                        self.tempo_repo.remove(&to.start_tick);
                        self.tempo_repo.add(from.start_tick, from.clone());
                    }
                    for (from, to) in changed.dumpers.iter() {
                        self.dumper_repo.remove(&to.start_tick);
                        self.dumper_repo.add(from.start_tick, from.clone());
                    }
                    for (from, to) in changed.softs.iter() {
                        self.soft_repo.remove(&to.start_tick);
                        self.soft_repo.add(from.start_tick, from.clone());
                    }

                    for n in removed.notes.iter() {
                        self.note_repo.add(n.start_tick(), n.clone());
                    }
                    for b in removed.bars.iter() {
                        self.bar_repo.add(b.start_tick, *b);
                    }
                    for t in removed.tempos.iter() {
                        self.tempo_repo.add(t.start_tick, *t);
                    }
                    for d in removed.dumpers.iter() {
                        self.dumper_repo.add(d.start_tick, *d);
                    }
                    for s in removed.softs.iter() {
                        self.soft_repo.add(s.start_tick, *s);
                    }
                },
                Undo::Removed(remove) => {
                    for n in remove.notes.iter() {
                        self.note_repo.add(n.start_tick(), n.clone());
                    }
                    for b in remove.bars.iter() {
                        self.bar_repo.add(b.start_tick, b.clone());
                    }
                    for t in remove.tempos.iter() {
                        self.tempo_repo.add(t.start_tick, t.clone());
                    }
                    for d in remove.dumpers.iter() {
                        self.dumper_repo.add(d.start_tick, d.clone());
                    }
                    for s in remove.softs.iter() {
                        self.soft_repo.add(s.start_tick, s.clone());
                    }
                }
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

    pub fn bar_events(&self) -> &Vec<StoreEvent<u32, Bar, Option<bool>>> {
        self.bar_repo.events()
    }

    pub fn tempo_events(&self) -> &Vec<StoreEvent<u32, Tempo, Option<bool>>> {
        self.tempo_repo.events()
    }

    pub fn dumper_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, Option<bool>>> {
        self.dumper_repo.events()
    }

    pub fn soft_events(&self) -> &Vec<StoreEvent<u32, CtrlChg, Option<bool>>> {
        self.soft_repo.events()
    }

    pub fn note_events(&self) -> &Vec<BagStoreEvent<u32, Rc<Note>, Option<bool>>> {
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
            self.add_bar(bar);
            replenished_bars.push(bar);
        }
        replenished_bars
    }
}

pub fn tempo_at(tick: u32, store: &Store<u32, Tempo, Option<bool>>) -> TempoValue {
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

pub fn ctrl_chg_at(tick: u32, store: &Store<u32, CtrlChg, Option<bool>>) -> Velocity {
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
}

impl Cmd for ProjectCmd {
    type Model = Project;

    fn undo(&self, proj: &mut Self::Model) {
        match self {
            ProjectCmd::SetRhythm(old_rhythm, _) => {
                proj.rhythm = *old_rhythm;
            },
        }
    }

    fn redo(&self, proj: &mut Self::Model) {
        match self {
            ProjectCmd::SetRhythm(_, new_rhythm) => {
                proj.rhythm = *new_rhythm;
            },
        }
    }
}

impl SerializableCmd for ProjectCmd {
}

trait ProjectIntf {
    fn set_rhythm(&mut self, rhythm: Rhythm) -> Result<(), SqliteUndoStoreAddCmdError>;
}

impl ProjectIntf for SqliteUndoStore::<ProjectCmd, Project> {
    fn set_rhythm(&mut self, rhythm: Rhythm) -> Result<(), SqliteUndoStoreAddCmdError> {
        self.add_cmd(Box::new(ProjectCmd::SetRhythm(self.model().rhythm, rhythm)))
    }
}

#[cfg(test)]
mod tests {
    use klavier_helper::store::Store;
    use serdo::undo_store::{SqliteUndoStore, UndoStore};

    use crate::{tempo::{Tempo, TempoValue}, project::{tempo_at, LocationError, ProjectCmd}, note::Note, solfa::Solfa, octave::Octave, sharp_flat::SharpFlat, pitch::Pitch, duration::{Duration, Numerator, Denominator, Dots}, velocity::Velocity, trimmer::{Trimmer, RateTrimmer}, bar::{Bar, DcFine, EndOrRegion, RepeatStart}, location::Location, rhythm::Rhythm, ctrl_chg::CtrlChg, key::Key};

    use super::{DEFAULT_TEMPO, Project};
    
    #[test]
    fn tempo() {
        let mut store: Store<u32, Tempo, Option<bool>> = Store::new(false);
        assert_eq!(tempo_at(0, &store), DEFAULT_TEMPO);

        store.add(10, Tempo { start_tick: 10, value: TempoValue::new(100) });
        assert_eq!(tempo_at(0, &store), DEFAULT_TEMPO);
        assert_eq!(tempo_at(10, &store), TempoValue::new(100));
        assert_eq!(tempo_at(11, &store), TempoValue::new(100));

        store.add(20, Tempo { start_tick: 20, value: TempoValue::new(200) });
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

        let _ = proj.add_note(note);
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
        proj.add_bar(bar);
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
        proj.add_bar(bar);

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
        proj.add_bar(bar);

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
        proj.add_bar(bar);

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
        proj.add_bar(bar);

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
        proj.add_bar(bar0);
        assert_eq!(proj.last_bar(), Some(bar0));

        assert_eq!(proj.rhythm_at(0), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(99), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(100), Rhythm::new(6, 8));
        assert_eq!(proj.rhythm_at(101), Rhythm::new(6, 8));

        let bar1 = Bar::new(
            200, Some(Rhythm::new(3, 4)),
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar1);
        assert_eq!(proj.last_bar(), Some(bar1));

        let bar2 = Bar::new(
            300, None,
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar2);
        assert_eq!(proj.last_bar(), Some(bar2));

        let bar3 = Bar::new(
            400, Some(Rhythm::new(4, 4)),
            None, DcFine::Null, EndOrRegion::Null, RepeatStart::Null
        );
        proj.add_bar(bar3);
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

        let note0 = proj.add_note(
            Note::new( // end tick: 100 + 240 * 1.5 = 460
                100, pitch,
                Duration::new(Numerator::Quarter, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO
            )
        );
        assert_eq!(proj.note_max_end_tick(), Some(note0.base_start_tick() + note0.tick_len()));

        let note1 = proj.add_note(
            Note::new( // end tick: 100 + 960 * 1.5 = 1540
                100, pitch,
                Duration::new(Numerator::Whole, Denominator::from_value(2).unwrap(), Dots::ONE),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 1.0),
                Trimmer::ZERO
            )
        );
        assert_eq!(proj.note_max_end_tick(), Some(note1.base_start_tick() + note1.tick_len()));

        let _ = proj.add_note(
            Note::new( // end tick: 200 + 120 * 2.5 = 440
                200, pitch,
                Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
                false, false, Velocity::new(10), Trimmer::ZERO,
                RateTrimmer::new(1.0, 1.0, 1.0, 2.0),
                Trimmer::ZERO
            )
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
            )
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
            )
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
            )
        );
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 3);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 3);

        let _ = proj.add_tempo(Tempo::new(960*3, 200));
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 3);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 3);

        let _ = proj.add_tempo(Tempo::new(960 * 3 + 1, 200));
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 4);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 4);

        let _ = proj.add_dumper(CtrlChg::new(960 * 4, Velocity::new(20)));
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 4);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 4);

        let _ = proj.add_dumper(CtrlChg::new(960 * 4 + 1, Velocity::new(20)));
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 5);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 5);

        let _ = proj.add_soft(CtrlChg::new(960 * 5, Velocity::new(20)));
        proj.replenish_bars();
        assert_eq!(proj.bar_repo().len(), 5);
        assert_eq!(proj.bar_repo().peek_last().unwrap().0, 960 * 5);

        let _ = proj.add_dumper(CtrlChg::new(960 * 5 + 1, Velocity::new(20)));
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
        let note0 = proj.add_note(note0);

        let note1 = Note::new(
            120,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        let note1 = proj.add_note(note1);

        let note2 = Note::new(
            150,
            Pitch::new(Solfa::C, Octave::Oct4, SharpFlat::Null),
            Duration::new(Numerator::N8th, Denominator::from_value(2).unwrap(), Dots::ZERO),
            false, false,
            Velocity::new(64),
            Trimmer::ZERO, RateTrimmer::ONE, Trimmer::ZERO
        );
        let note2 = proj.add_note(note2);
        assert_eq!(proj.note_repo().len(), 3);

        proj.tuplize(vec![note0.clone(), note1.clone(), note2.clone()]);
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
        proj.set_key(Key::FLAT_2);
        proj.rhythm = Rhythm::new(3, 4);

        let ser = bincode::serialize(&proj).unwrap();

        let des: Project = bincode::deserialize(&ser).unwrap();

        assert_eq!(proj.key, des.key);
        assert_eq!(proj.rhythm, des.rhythm);
    }

    #[test]
    fn sqlite_store_can_work() {
        use tempfile::tempdir;
        use super::ProjectIntf;

        let dir = tempdir().unwrap();
        let mut dir = dir.as_ref().to_path_buf();
        dir.push("project");
        let mut store = SqliteUndoStore::<ProjectCmd, Project>::open(dir.clone(), None).unwrap();

        store.set_rhythm(Rhythm::new(12, 8)).unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));

        store.set_rhythm(Rhythm::new(12, 4)).unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 4));

        store.undo().unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));

        store.undo().unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::default());

        store.redo().unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));

        store.set_rhythm(Rhythm::new(16, 8)).unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(16, 8));

        store.undo().unwrap();
        assert_eq!(store.model().rhythm(), Rhythm::new(12, 8));
    }
}