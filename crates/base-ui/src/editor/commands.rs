//! Editing commands on the selection: move, delete, copy and paste, align, flip, rotate, order, group and lock.
use super::EditorHost;
use crate::i18n::{tr, trf};
use base_core::geom::{Pt, Rect, Xf};
use base_core::scene::Shape;

/// The editing commands. Implemented for every `EditorHost`.
pub trait EditorCommands: EditorHost {
    fn transform_selection(&mut self, xf: Xf) {
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                s.xf = s.xf.then(xf);
            }
        }
        self.touch();
    }

    fn delete_selection(&mut self) {
        if self.ed().sel.is_empty() {
            return;
        }
        self.checkpoint();
        let sel = std::mem::take(&mut self.ed_mut().sel);
        // Locked shapes survive and stay selected.
        self.scene_mut().shapes.retain(|s| !sel.contains(&s.id) || s.locked);
        self.ed_mut().sel = sel.into_iter().filter(|id| self.scene().shape(*id).is_some()).collect();
    }

    fn copy(&mut self) {
        // A copy of a locked object is a normal, editable object.
        let copy = self.ed().sel.iter().filter_map(|i| self.scene().shape(*i).cloned()).map(|mut s| { s.locked = false; s }).collect();
        self.ed_mut().clipboard = copy;
    }

    fn paste(&mut self) {
        if self.ed().clipboard.is_empty() {
            return;
        }
        self.checkpoint();
        self.ed_mut().sel.clear();
        // Pasted groups get their own group ids so they do not merge with the originals.
        let mut gmap: std::collections::HashMap<u64, u64> = Default::default();
        for mut s in self.ed().clipboard.clone() {
            s.xf = s.xf.then(Xf::translate(5.0, 5.0));
            if let Some(g) = s.group {
                let fresh = match gmap.get(&g) {
                    Some(n) => *n,
                    None => {
                        let n = self.scene_mut().new_group_id();
                        gmap.insert(g, n);
                        n
                    }
                };
                s.group = Some(fresh);
            }
            let id = self.scene_mut().add_shape(s);
            self.ed_mut().sel.push(id);
        }
        // Next paste lands a bit further away.
        let next = self.ed().sel.iter().filter_map(|i| self.scene().shape(*i).cloned()).collect();
        self.ed_mut().clipboard = next;
    }

    fn duplicate(&mut self) {
        self.copy();
        self.paste();
    }

    fn select_all(&mut self) {
        let vis: Vec<u64> = self.scene().shapes.iter().filter(|s| self.layer_visible(s.layer)).map(|s| s.id).collect();
        self.ed_mut().sel = vis;
    }

    fn align(&mut self, mode: u8) {
        let Some(b) = self.sel_bounds() else { return };
        // A single object aligns to the bed, several align to each other.
        let target = if self.ed().sel.len() == 1 {
            Rect { min: Pt::new(0.0, 0.0), max: { let (w, h) = self.work_size(); Pt::new(w, h) } }
        } else {
            b
        };
        self.checkpoint();
        for id in self.ed().sel.clone() {
            let Some(sb) = self.scene().shape(id).and_then(|s| s.bounds()) else { continue };
            let (dx, dy) = match mode {
                0 => (target.min.x - sb.min.x, 0.0),
                1 => (target.center().x - sb.center().x, 0.0),
                2 => (target.max.x - sb.max.x, 0.0),
                3 => (0.0, target.min.y - sb.min.y),
                4 => (0.0, target.center().y - sb.center().y),
                _ => (0.0, target.max.y - sb.max.y),
            };
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                s.xf = s.xf.then(Xf::translate(dx, dy));
            }
        }
    }

    /// Move the selected objects so that their centres coincide, at the centre of the whole selection. A group
    /// counts as one object. With the small shape inside the large one, the large one stays where it is.
    fn center_on_each_other(&mut self) {
        let Some(all) = self.sel_bounds() else { return };
        let target = all.center();
        // Objects are single shapes or whole groups.
        let mut units: Vec<Vec<u64>> = vec![];
        for id in self.ed().sel.clone() {
            if units.iter().any(|u| u.contains(&id)) {
                continue;
            }
            units.push(self.scene().group_of(id));
        }
        if units.len() < 2 {
            return;
        }
        self.checkpoint();
        for unit in units {
            let Some(b) = self.scene().bounds_of(&unit) else { continue };
            let t = Xf::translate(target.x - b.center().x, target.y - b.center().y);
            for id in unit {
                if let Some(s) = self.scene_mut().unlocked_mut(id) {
                    s.xf = s.xf.then(t);
                }
            }
        }
    }

    fn flip(&mut self, horizontal: bool) {
        if let Some(b) = self.sel_bounds() {
            self.checkpoint();
            let c = b.center();
            self.transform_selection(if horizontal { Xf::scale_about(-1.0, 1.0, c) } else { Xf::scale_about(1.0, -1.0, c) });
        }
    }

    fn rotate_sel(&mut self, deg: f64) {
        if let Some(b) = self.sel_bounds() {
            self.checkpoint();
            self.transform_selection(Xf::rotate_about(deg.to_radians(), b.center()));
        }
    }

    fn center_on_bed(&mut self) {
        if let Some(b) = self.sel_bounds() {
            self.checkpoint();
            let c = b.center();
            let (bw, bh) = self.work_size();
            let t = Xf::translate(bw / 2.0 - c.x, bh / 2.0 - c.y);
            self.transform_selection(t);
        }
    }

    fn to_path(&mut self) {
        if self.ed().sel.is_empty() {
            return;
        }
        self.checkpoint();
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                s.bake();
            }
        }
    }

    fn reorder(&mut self, front: bool) {
        self.checkpoint();
        let sel = self.scene().unlocked_ids(&self.ed().sel);
        let (a, b): (Vec<Shape>, Vec<Shape>) = std::mem::take(&mut self.scene_mut().shapes).into_iter().partition(|s| sel.contains(&s.id));
        self.scene_mut().shapes = if front { [b, a].concat() } else { [a, b].concat() };
    }

    /// Group the selected shapes. Layers are untouched; a group moves, scales and selects as one.
    fn group_selection(&mut self) {
        if self.ed().sel.len() < 2 {
            self.say(tr("Select at least two objects to group.").to_string());
            return;
        }
        self.checkpoint();
        let g = self.scene_mut().new_group_id();
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().shape_mut(id) {
                s.group = Some(g);
            }
        }
        self.say(tr("Grouped. The objects keep their layers.").to_string());
    }

    /// Lock or unlock the selected objects (a group is selected as a whole, so it locks as a whole).
    fn lock_selection(&mut self, lock: bool) {
        if !self.ed().sel.iter().any(|id| self.scene().shape(*id).is_some_and(|s| s.locked != lock)) {
            return;
        }
        self.checkpoint();
        let mut n = 0;
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().shape_mut(id) {
                if s.locked != lock {
                    s.locked = lock;
                    n += 1;
                }
            }
        }
        self.ed_mut().node_sel.clear();
        self.say(if lock { trf("Locked {} object(s).", &[&n]) } else { trf("Unlocked {} object(s).", &[&n]) });
    }

    fn ungroup_selection(&mut self) {
        if !self.ed().sel.iter().any(|id| self.scene().shape(*id).is_some_and(|s| s.group.is_some())) {
            return;
        }
        self.checkpoint();
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().shape_mut(id) {
                s.group = None;
            }
        }
    }
}

impl<T: EditorHost> EditorCommands for T {}
