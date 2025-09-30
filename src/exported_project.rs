use serde_json::Value;

use crate::{bar::Bar, ctrl_chg::CtrlChg, key::Key, models::{FromClipboardTextErr, Models}, note::Note, rhythm::Rhythm, tempo::Tempo};

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

impl ExportedProject {
    pub fn from_clipboard_text(json: String) -> Result<Self, FromClipboardTextErr> {
        let mut stream = serde_json::Deserializer::from_str(&json).into_iter::<Value>();
        match stream.next() {
            None => Err(FromClipboardTextErr::EmptyString),
            Some(Ok(ver)) =>
                if let Value::Number(ver_no) = ver {
                    if let Some(v) = ver_no.as_u64() {
                        if v == Models::VERSION {
                            serde_json::from_slice::<'_, ExportedProject>(&json.as_bytes()[stream.byte_offset()..])
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

    pub fn to_models(self) -> Models {
        Models {
            notes: self.notes,
            bars: self.bars,
            tempos: self.tempos,
            dumpers: self.dumpers,
            softs: self.softs,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{key::Key, rhythm::Rhythm};
    use super::ExportedProject;

    #[test]
    fn import_key() {
        let proj = ExportedProject {
            key: Some(Key::FLAT_3),
            rhythm: None,
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![]
        };
        let json = format!("1{}", serde_json::to_string(&proj).unwrap());
        
        let deserialized = ExportedProject::from_clipboard_text(json).unwrap();
        assert_eq!(deserialized.key, Some(Key::FLAT_3));
    }

    #[test]
    fn import_rhythm() {
        let proj = ExportedProject {
            key: None,
            rhythm: Some(Rhythm::new(3, 4)),
            notes: vec![],
            bars: vec![],
            tempos: vec![],
            dumpers: vec![],
            softs: vec![]
        };
        let json = format!("1{}", serde_json::to_string(&proj).unwrap());

        let deserialized = ExportedProject::from_clipboard_text(json).unwrap();
        assert_eq!(deserialized.rhythm, Some(Rhythm::new(3, 4)));
    }
}