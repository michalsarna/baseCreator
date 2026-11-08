//! The library of drag knives: the usual blades (built in) and the knives of one's own, each with the offset of its tip, the corners
//! it turns round and its overcut. A device or a layer takes the values of a knife of the library and keeps its own copy.
use crate::drag_knife::Knife;
use serde::{Deserialize, Serialize};

/// A knife of the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blade {
    /// Stays the same when the knife is renamed.
    pub id: String,
    pub name: String,
    pub knife: Knife,
    #[serde(default)]
    pub notes: String,
}

/// The built-in knives and the ones of one's own.
#[derive(Clone, Debug, PartialEq)]
pub struct KnifeLibrary {
    pub builtin: Vec<Blade>,
    pub user: Vec<Blade>,
}

/// The usual blades (id, name, offset, swivel, overcut, notes): the angle of the blade sets how far its tip trails behind the
/// axis. The names and notes are English; the programs show them translated.
pub const BUILTIN: [(&str, &str, f64, f64, f64, &str); 4] = [
    ("std-30", "30° blade", 0.15, 15.0, 0.4, "Thin film and paper, fine detail and small letters."),
    ("std-45", "45° blade", 0.25, 15.0, 0.5, "Vinyl, flex, paper and thin card: the usual blade of a cutter."),
    ("std-60", "60° blade", 0.5, 15.0, 0.8, "Thick vinyl, card, rubber, foam and sandblast stencils."),
    ("cnc-holder", "Drag knife in a holder of a CNC machine", 0.5, 10.0, 1.0, "A holder in the collet of the spindle, the spindle still; set the depth on the layer."),
];

pub fn builtin_knives() -> Vec<Blade> {
    BUILTIN.iter().map(|&(id, name, offset, swivel_deg, overcut, notes)| Blade { id: id.into(), name: name.into(), knife: Knife { offset, swivel_deg, overcut }, notes: notes.into() }).collect()
}

impl Default for KnifeLibrary {
    fn default() -> KnifeLibrary {
        KnifeLibrary { builtin: builtin_knives(), user: vec![] }
    }
}

impl KnifeLibrary {
    /// Every knife: the built-in ones first.
    pub fn all(&self) -> impl Iterator<Item = &Blade> {
        self.builtin.iter().chain(self.user.iter())
    }

    pub fn get(&self, id: &str) -> Option<&Blade> {
        self.all().find(|b| b.id == id)
    }

    pub fn is_builtin(&self, id: &str) -> bool {
        self.builtin.iter().any(|b| b.id == id)
    }

    /// An id not used yet.
    pub fn new_id(&self) -> String {
        (1..).map(|n| format!("knife-{n}")).find(|id| self.get(id).is_none()).expect("a free id")
    }

    /// A new knife of one's own, a copy of `from` (or the 45° blade), with its id.
    pub fn add_copy(&mut self, from: Option<&str>, name: &str) -> String {
        let base = from.and_then(|id| self.get(id)).cloned().unwrap_or_else(|| builtin_knives()[1].clone());
        let id = self.new_id();
        self.user.push(Blade { id: id.clone(), name: name.into(), ..base });
        id
    }

    pub fn remove(&mut self, id: &str) {
        self.user.retain(|b| b.id != id);
    }

    /// The knives of one's own as text (the built-in ones are always there).
    pub fn save_text(&self) -> String {
        serde_json::to_string(&self.user).unwrap_or_default()
    }

    /// Take the knives of one's own from text; broken text is left out.
    pub fn load_text(&mut self, text: &str) {
        if let Ok(user) = serde_json::from_str::<Vec<Blade>>(text) {
            self.user = user.into_iter().filter(|b| !self.builtin.iter().any(|x| x.id == b.id)).collect();
        }
    }

    /// The knife of the library whose values these are, if any.
    pub fn matching(&self, k: &Knife) -> Option<&Blade> {
        self.all().find(|b| (b.knife.offset - k.offset).abs() < 1e-9 && (b.knife.swivel_deg - k.swivel_deg).abs() < 1e-9 && (b.knife.overcut - k.overcut).abs() < 1e-9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knives_of_ones_own_are_kept_and_the_built_in_ones_always_there() {
        let mut lib = KnifeLibrary::default();
        assert_eq!(lib.all().count(), 4);
        let id = lib.add_copy(Some("std-60"), "My rubber knife");
        assert_eq!(lib.get(&id).unwrap().knife.offset, 0.5);
        lib.user[0].knife.offset = 0.6;
        let text = lib.save_text();
        let mut back = KnifeLibrary::default();
        back.load_text(&text);
        assert_eq!(back.user, lib.user);
        assert_eq!(back.matching(&Knife { offset: 0.25, swivel_deg: 15.0, overcut: 0.5 }).map(|b| b.id.as_str()), Some("std-45"));
        back.load_text("not json");
        assert_eq!(back.user.len(), 1, "broken text changes nothing");
        back.remove(&id);
        assert!(back.user.is_empty());
        assert!(lib.is_builtin("std-30") && !lib.is_builtin(&id));
    }
}
