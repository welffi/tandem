use crate::app::App;
use crate::assets::probe_asset;
use crate::jobs::{Done, Reversed};
use crate::media::reversed_media;
use crate::reverse::{Range, Swap, apply_swaps, plan_reverse, reversed_asset};
use crate::types::MediaAsset;

fn reverse_range(range: Range, source: &MediaAsset, known: &[MediaAsset]) -> Result<(Range, MediaAsset), String> {
    let made = reversed_media(&source.source_path, range.in_sec, range.out_sec, range.video).and_then(|path| probe_asset(&path.to_string_lossy(), known));
    made.map(|asset| (range, asset)).map_err(|e| format!("Could not reverse {}: {e}", source.name))
}

impl App {
    pub fn reverse_selection(&mut self) {
        let project = self.store.project().clone();
        let plan = plan_reverse(&project, &self.store.selection);
        if !plan.swaps.is_empty() {
            self.store.edit(apply_swaps(&project, Vec::new(), &plan.swaps), None);
        }
        if plan.ranges.is_empty() {
            return;
        }
        self.store.notify(if plan.ranges.len() == 1 { "Reversing the clip...".to_string() } else { format!("Reversing {} clips...", plan.ranges.len()) });
        let work: Vec<_> = plan.ranges.into_iter().filter_map(|r| project.assets.iter().find(|a| a.id == r.asset_id).cloned().map(|source| (r, source))).collect();
        let known = project.assets;
        self.jobs.run(move || Done::Reversed(work.into_iter().map(|(range, source)| reverse_range(range, &source, &known)).collect::<Vec<Reversed>>()));
    }

    pub fn finish_reverse(&mut self, results: Vec<Reversed>) {
        let mut added = Vec::new();
        let mut swaps = Vec::new();
        for result in results {
            match result {
                Ok((range, made)) => {
                    let Some(source) = self.store.project().assets.iter().find(|a| a.id == range.asset_id) else { continue };
                    let asset = reversed_asset(made, source, &range);
                    swaps.push(Swap { clip_ids: range.clip_ids, asset_id: asset.id.clone(), pivot_sec: range.out_sec });
                    added.push(asset);
                }
                Err(message) => self.store.notify(message),
            }
        }
        if swaps.is_empty() {
            return;
        }
        let next = apply_swaps(self.store.project(), added.clone(), &swaps);
        self.store.edit(next, None);
        added.iter().for_each(|a| self.load_editor_data(a));
    }
}
