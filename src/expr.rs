use std::collections::BTreeMap;
use fasteval::{self, Compiler, Evaler};
use crate::expr::fasteval::eval_compiled_ref;
use crate::note::Note;

#[derive(Debug)]
pub struct Expr {
  slab: fasteval::Slab,
  compiled: fasteval::Instruction,
}

impl Expr {
  pub fn new<S: AsRef<str>>(expr: S) -> Result<Self, fasteval::Error>  {
    let parser = fasteval::Parser::new();
    let mut slab = fasteval::Slab::new();
    let compiled: fasteval::Instruction = parser.parse(expr.as_ref(), &mut slab.ps)?.from(&slab.ps).compile(&slab.ps, &mut slab.cs);

    Ok(Self { slab, compiled })
  }
  
  pub fn evaluate_note(&self, note: &Note) -> Result<bool, fasteval::Error>{
    let mut map: BTreeMap<&str, f64> = BTreeMap::new();
    map.insert("v", note.velocity().as_u8() as f64); // velocity
    map.insert("bv", note.base_velocity.as_u8() as f64); // base velocity
    map.insert("vt0", note.velocity_trimmer.value(0) as f64); // velocity trimmer 0
    map.insert("vt1", note.velocity_trimmer.value(1) as f64); // velocity trimmer 1
    map.insert("vt2", note.velocity_trimmer.value(2) as f64); // velocity trimmer 2
    map.insert("vt3", note.velocity_trimmer.value(3) as f64); // velocity trimmer 3
    map.insert("vt", note.velocity_trimmer.sum() as f64); // sum of velocity trimmers
    
    let val: f64 = eval_compiled_ref!(&self.compiled, &self.slab, &mut map);

    Ok(val != 0.)
  }
}

#[cfg(test)]
mod tests {
    use crate::{note::{Note, NoteBuilder}, trimmer::Trimmer, velocity::Velocity};
    use super::Expr;

  #[test]
  fn eval_note_velocity() {
    let expr0: Expr = Expr::new("bv < 64").unwrap();
    let expr1: Expr = Expr::new("vt0 < 20").unwrap();
    let expr2: Expr = Expr::new("vt0 <= 20").unwrap();
    let expr3: Expr = Expr::new("vt < 20").unwrap();
    let expr4: Expr = Expr::new("vt <= 20").unwrap();
    let note0: Note = NoteBuilder::default()
        .base_velocity(Velocity::new(64))
        .velocity_trimmer(Trimmer::new(20, 0, 0, 0))
        .build().unwrap();
    assert_eq!(expr0.evaluate_note(&note0).unwrap(), false);
    assert_eq!(expr1.evaluate_note(&note0).unwrap(), false);
    assert_eq!(expr2.evaluate_note(&note0).unwrap(), true);
    assert_eq!(expr3.evaluate_note(&note0).unwrap(), false);
    assert_eq!(expr4.evaluate_note(&note0).unwrap(), true);

    let note1: Note = NoteBuilder::default()
        .base_velocity(Velocity::new(63))
        .velocity_trimmer(Trimmer::new(10, 10, 0, 0))
        .build().unwrap();
    assert_eq!(expr0.evaluate_note(&note1).unwrap(), true);
    assert_eq!(expr1.evaluate_note(&note1).unwrap(), true);
    assert_eq!(expr2.evaluate_note(&note1).unwrap(), true);
    assert_eq!(expr3.evaluate_note(&note1).unwrap(), false);
    assert_eq!(expr4.evaluate_note(&note1).unwrap(), true);

    assert_eq!(Expr::new("foo <= 20").unwrap().evaluate_note(&note1), Err(fasteval::Error::Undefined("foo".to_owned())));
  }
}
