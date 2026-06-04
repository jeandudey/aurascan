pub use self::bbox::{BoundingBox, Detection};
pub use self::strack::STrack;
use nalgebra::{DMatrix, SMatrix, SVector};
use std::collections::HashSet;

mod bbox;
mod kalman;
mod strack;

type State = SVector<f32, 8>;
type Covariance = SMatrix<f32, 8, 8>;
type Measurement = SVector<f32, 4>;

#[derive(Debug)]
struct Counter(usize);

impl Counter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn with_start(start: usize) -> Self {
        Self(start)
    }

    pub fn increment(&mut self) -> usize {
        let id = self.0;
        self.0 += 1;
        id
    }
}

pub struct Settings {
    pub track_threshold: f32,
    pub low_threshold: f32,
    pub det_threshold: f32,
    pub match_threshold: f32,
    pub max_time_lost: usize,
}

pub struct ByteTrack {
    settings: Settings,
    frame_id: Counter,
    next_id: Counter,
    tracked_stracks: Vec<STrack>,
    lost_stracks: Vec<STrack>,
    removed_stracks: Vec<STrack>,
}

impl ByteTrack {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            frame_id: Counter::with_start(1),
            next_id: Counter::new(),
            tracked_stracks: Vec::new(),
            lost_stracks: Vec::new(),
            removed_stracks: Vec::new(),
        }
    }

    pub fn update(&mut self, detections: &[Detection]) -> Vec<STrack> {
        let frame_id = self.frame_id.increment();

        let mut activated_stracks = Vec::new();
        let mut refind_stracks = Vec::new();
        let mut removed_stracks = Vec::new();
        let mut lost_stracks = Vec::new();

        // Split detections into high and low confidence tracklets.
        let (high, low) = split_detections(detections, self.settings.track_threshold);

        // Add newly detected tracklets to tracked_stracks.
        let mut unconfirmed = Vec::new();
        let mut tracked_stracks = Vec::new();
        for track in self.tracked_stracks.iter().cloned() {
            if !track.is_activated() {
                unconfirmed.push(track);
            } else {
                tracked_stracks.push(track);
            }
        }
        for track in unconfirmed.iter_mut() {
            track.set_det_idx(None);
        }

        // First association with high score tracklets.
        let mut strack_pool = joint_stracks(&tracked_stracks, &self.lost_stracks);
        for strack in strack_pool.iter_mut() {
            strack.set_det_idx(None);
            strack.predict();
        }
        let (unmatched_stracks, unmatched_high) = association(
            &strack_pool,
            &high,
            self.settings.match_threshold,
            frame_id,
            &mut activated_stracks,
            &mut refind_stracks,
        );

        // Second association with low score tracklets.
        let remaining_tracked_stracks: Vec<_> = unmatched_stracks
            .into_iter()
            .filter_map(|i| {
                let track = &strack_pool[i];
                if track.is_tracked() {
                    Some(track.clone())
                } else {
                    None
                }
            })
            .collect();
        let (unmatched_remaining_tracked_stracks, _unmatched_low) = association(
            &remaining_tracked_stracks,
            &low,
            0.5,
            frame_id,
            &mut activated_stracks,
            &mut refind_stracks,
        );

        lost_stracks.extend(
            unmatched_remaining_tracked_stracks
                .into_iter()
                .filter_map(|i| {
                    let track = &remaining_tracked_stracks[i];
                    if track.is_lost() {
                        let mut track = track.clone();
                        track.mark_lost();
                        Some(track.clone())
                    } else {
                        None
                    }
                }),
        );

        // Deal with unconfirmed tracks, usually with only one beginning frame.
        let detections_high: Vec<_> = unmatched_high
            .into_iter()
            .map(|i| high[i].clone())
            .collect();
        let dists = iou_distance(&unconfirmed, &detections_high);
        let (matches, unmatched_unconfirmed, unmatched_remaining_high) =
            linear_assignment(&dists, 0.7);

        for (track_idx, detection_idx) in matches {
            unconfirmed[track_idx].update(&detections_high[detection_idx], frame_id);
            activated_stracks.push(unconfirmed[track_idx].clone());
        }

        for track_idx in unmatched_unconfirmed {
            let mut track = unconfirmed[track_idx].clone();
            track.mark_removed();
            removed_stracks.push(track);
        }

        // Init new stracks
        for detection_idx in unmatched_remaining_high {
            let mut track = detections_high[detection_idx].clone();
            if track.score() < self.settings.det_threshold {
                continue;
            }
            track.activate(&mut self.next_id, frame_id);
            activated_stracks.push(track);
        }

        // Update state
        for track in self.lost_stracks.iter() {
            let mut track = track.clone();
            if self.frame_id.0 - track.frame_id() > self.settings.max_time_lost {
                track.mark_removed();
                removed_stracks.push(track);
            }
        }

        self.tracked_stracks.retain(STrack::is_tracked);
        self.tracked_stracks = joint_stracks(&self.tracked_stracks, &activated_stracks);
        self.tracked_stracks = joint_stracks(&self.tracked_stracks, &refind_stracks);
        self.lost_stracks = sub_stracks(&self.lost_stracks, &self.tracked_stracks);
        self.lost_stracks.extend(lost_stracks);
        self.lost_stracks = sub_stracks(&self.lost_stracks, &self.removed_stracks);
        self.removed_stracks.extend(removed_stracks);

        let (tracked_stracks, lost_stracks) =
            remove_duplicate_stracks(&self.tracked_stracks, &self.lost_stracks);
        self.tracked_stracks = tracked_stracks;
        self.lost_stracks = lost_stracks;

        self.tracked_stracks
            .iter()
            .filter(|track| track.is_activated())
            .cloned()
            .collect()
    }

    /*
    pub fn update(&mut self, detections: &[Detection]) {

        let mut lost_stracks = Vec::new();
        let mut removed_stracks = Vec::new();


        let mut strack_pool = joint_stracks(&tracked_stracks, &self.lost_stracks);
        for strack in strack_pool.iter_mut() {
            strack.predict();
        }

        let dists_high = iou_distance(&tracked_stracks, &high);
        let (matches_high, u_track_high, u_detection_high) =
            linear_assignment(&dists_high, self.settings.match_threshold);

        for (itracked, idet) in matches_high {
            let track = &strack_pool[itracked];
            let det = &high[idet];
            if track.state() == TrackState::Tracked {
                todo!()
            } else {
                todo!()
            }
        }

        let r_tracked_stracks = tracked_stracks
            .iter()
            .filter(|track| track.state() == TrackState::Tracked)
            .cloned()
            .collect::<Vec<_>>();
        let dists_low = iou_distance(&r_tracked_stracks, &low);
        let (matches_low, u_track_low, u_detection_low) = linear_assignment(&dists_low, 0.5);
        for (itracked, idet) in matches_low {
            let track = &r_tracked_stracks[itracked];
            let det = &low[idet];
            if track.state() == TrackState::Tracked {
                todo!()
            } else {
                todo!()
            }
        }

        for it in u_track_low {
            let track = &r_tracked_stracks[it];
            if track.state() != TrackState::Lost {
                let mut track = track.clone();
                track.mark_lost();
                lost_stracks.push(track);
            }
        }

        let detections: Vec<_> = u_detection_high.iter().map(|&i| high[i].clone()).collect();
        let dists = iou_distance(&unconfirmed, &detections);
        let (matches, u_unconfirmed, u_detection) = linear_assignment(&dists, 0.7);
        for (itracked, idet) in matches {
            todo!("0.7")
        }
        for it in u_unconfirmed {
            let mut track = unconfirmed[it].clone();
            track.mark_removed();
            removed_stracks.push(track);
        }

        for inew in u_detection {
            let track = &detections[inew];
            if track.score < self.settings.det_threshold {
                continue;
            }
            todo!("activate")
        }
    }

    pub fn update(&mut self, detections: &[Detection]) -> Vec<(usize, BoundingBox)> {
        self.frame_id += 1;
        let mut activated = Vec::new();
        let mut refind = Vec::new();
        let mut lost = Vec::new();
        let mut removed = Vec::new();

        let high: Vec<usize> = (0..detections.len())
            .filter(|&i| detections[i].score >= self.settings.track_threshold)
            .collect();
        let low: Vec<usize> = (0..detections.len())
            .filter(|&i| {
                detections[i].score >= self.settings.low_threshold
                    && detections[i].score < self.settings.track_threshold
            })
            .collect();

        let mut unconfirmed: Vec<STrack> = Vec::new();
        let mut tracked: Vec<STrack> = Vec::new();
        for track in std::mem::take(&mut self.tracked_stracks) {
            if track.is_activated() {
                tracked.push(track);
            } else {
                unconfirmed.push(track);
            }
        }

        let mut strack_pool = joint_stracks(&tracked, &self.lost_stracks);
        for track in strack_pool.iter_mut() {
            track.predict();
        }

        let (m1, u_track1, u_high) = associate(
            &strack_pool,
            detections,
            &high,
            self.settings.match_threshold,
        );
        for (ti, di) in m1 {
            let det = &detections[di];
            if strack_pool[ti].state() == TrackState::Tracked {
                strack_pool[ti].update(det, self.frame_id);
                activated.push(strack_pool[ti].clone());
            } else {
            }
        }

        todo!()
    }
    */
}

/// Split detections by their score.
fn split_detections(detections: &[Detection], threshold: f32) -> (Vec<STrack>, Vec<STrack>) {
    let high = detections
        .iter()
        .enumerate()
        .filter(|(_, det)| det.score > threshold)
        .map(|(idx, det)| STrack::from_detection(det, idx))
        .collect();
    let low = detections
        .iter()
        .enumerate()
        .filter(|(_, det)| det.score > 0.1 && det.score <= threshold)
        .map(|(idx, det)| STrack::from_detection(det, idx))
        .collect();

    (high, low)
}

fn joint_stracks(a: &[STrack], b: &[STrack]) -> Vec<STrack> {
    let mut joint = a.to_vec();
    let mut seen: HashSet<usize> = joint.iter().map(|track| track.track_id()).collect();
    for track in b {
        if seen.insert(track.track_id()) {
            joint.push(track.clone());
        }
    }
    joint
}

fn sub_stracks(a: &[STrack], b: &[STrack]) -> Vec<STrack> {
    let b_ids: HashSet<_> = b.iter().map(|t| t.track_id()).collect();
    a.iter()
        .filter(|t| !b_ids.contains(&t.track_id()))
        .cloned()
        .collect()
}

fn remove_duplicate_stracks(a: &[STrack], b: &[STrack]) -> (Vec<STrack>, Vec<STrack>) {
    let dists = iou_distance(a, b);

    let mut dup_a: HashSet<usize> = HashSet::new();
    let mut dup_b: HashSet<usize> = HashSet::new();

    for p in 0..dists.nrows() {
        for q in 0..dists.ncols() {
            if dists[(p, q)] < 0.15 {
                let timep = a[p].frame_id() - a[p].start_frame();
                let timeq = b[q].frame_id() - b[q].start_frame();
                if timep > timeq {
                    dup_b.insert(q);
                } else {
                    dup_a.insert(p);
                }
            }
        }
    }

    let resa = a
        .iter()
        .enumerate()
        .filter(|(i, _)| !dup_a.contains(i))
        .map(|(_, t)| t)
        .cloned()
        .collect();
    let resb = b
        .iter()
        .enumerate()
        .filter(|(i, _)| !dup_b.contains(i))
        .map(|(_, t)| t)
        .cloned()
        .collect();

    (resa, resb)
}

fn iou_distance(a: &[STrack], b: &[STrack]) -> DMatrix<f32> {
    DMatrix::from_fn(a.len(), b.len(), |i, j| {
        1.0 - a[i].to_tlbr().iou(&b[j].to_tlbr())
    })
}

fn linear_assignment(
    cost: &DMatrix<f32>,
    threshold: f32,
) -> (Vec<(usize, usize)>, Vec<usize>, Vec<usize>) {
    use lapjv::{Matrix, lapjv};

    let (nrows, ncols) = cost.shape();
    if nrows == 0 || ncols == 0 {
        return (vec![], (0..nrows).collect(), (0..ncols).collect());
    }

    let n = nrows.max(ncols);
    let big = threshold + 1.0;
    let costs = Matrix::from_shape_fn((n, n), |(r, c)| {
        if r < nrows && c < ncols {
            cost[(r, c)]
        } else {
            big
        }
    });
    let result = lapjv(&costs).unwrap();
    let x = result.0;

    let mut matches = Vec::new();
    let mut unmatched_a = Vec::new();
    for r in 0..nrows {
        let c = x[r];
        if c < ncols && cost[(r, c)] <= threshold {
            matches.push((r, c));
        } else {
            unmatched_a.push(r);
        }
    }

    let matched_cols: HashSet<usize> = matches.iter().map(|&(_, c)| c).collect();
    let unmatched_b: Vec<usize> = (0..ncols).filter(|c| !matched_cols.contains(c)).collect();
    (matches, unmatched_a, unmatched_b)
}

fn association(
    stracks: &[STrack],
    detections: &[STrack],
    threshold: f32,
    frame_id: usize,
    activated_stracks: &mut Vec<STrack>,
    refind_stracks: &mut Vec<STrack>,
) -> (Vec<usize>, Vec<usize>) {
    let dists = iou_distance(stracks, detections);
    let (matches, unmatched_tracks, unmatched_detection) = linear_assignment(&dists, threshold);
    let matches = matches
        .into_iter()
        .map(|(track_idx, detection_idx)| (&stracks[track_idx], &detections[detection_idx]));
    for (track, detection) in matches {
        let mut track = track.clone();
        track.set_det_idx(detection.det_idx());
        if track.is_tracked() {
            track.update(detection, frame_id);
            activated_stracks.push(track);
        } else {
            track.reactivate(detection, frame_id, None);
            refind_stracks.push(track);
        }
    }
    (unmatched_tracks, unmatched_detection)
}
