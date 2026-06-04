use crate::bbox::{Detection, Tlbr, Tlwh};
use crate::{Counter, Covariance, State, kalman};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackState {
    Tracked,
    Lost,
    Removed,
}

#[derive(Debug, Clone)]
pub struct ActiveSTrack {
    score: f32,

    mean: State,
    covariance: Covariance,

    track_id: usize,
    frame_id: usize,
    start_frame: usize,

    tracklet_len: usize,
    state: TrackState,

    det_idx: Option<usize>,
}

#[derive(Debug, Clone)]
enum STrackInner {
    Inactive {
        bbox: Tlwh,
        score: f32,
        det_idx: Option<usize>,
    },
    Active(ActiveSTrack),
}

#[derive(Debug, Clone)]
pub struct STrack {
    inner: STrackInner,
}

impl STrack {
    /// Construct a new inactive `STrack` from a detection.
    pub(crate) fn from_detection(detection: &Detection, idx: usize) -> Self {
        Self {
            inner: STrackInner::Inactive {
                bbox: detection.to_tlwh(),
                score: detection.score,
                det_idx: Some(idx),
            },
        }
    }

    pub(crate) fn predict(&mut self) {
        self.with_active_mut(|strack| {
            let is_tracked = strack.state == TrackState::Tracked;
            if is_tracked {
                strack.mean[7] = 0.0;
            }
            kalman::predict(&mut strack.mean, &mut strack.covariance);
        });
    }

    pub(crate) fn activate(&mut self, track_id: &mut Counter, frame_id: usize) {
        match &self.inner {
            STrackInner::Active(_) => panic!("STrack is already active"),
            STrackInner::Inactive {
                bbox,
                score,
                det_idx,
            } => {
                let xyah = bbox.to_xyah();
                let (mean, covariance) = kalman::initiate(&xyah);
                let strack = ActiveSTrack {
                    score: *score,
                    mean,
                    covariance,
                    track_id: track_id.increment(),
                    frame_id,
                    start_frame: frame_id,
                    tracklet_len: 0,
                    state: TrackState::Tracked,
                    det_idx: *det_idx,
                };
                self.inner = STrackInner::Active(strack);
            }
        }
    }

    pub(crate) fn reactivate(
        &mut self,
        new: &STrack,
        frame_id: usize,
        new_id: Option<&mut Counter>,
    ) {
        self.with_active_mut(|track| {
            kalman::update(
                &mut track.mean,
                &mut track.covariance,
                &new.to_tlwh().to_xyah(),
            );

            track.tracklet_len = 0;
            track.state = TrackState::Tracked;
            track.frame_id = frame_id;
            if let Some(new_id) = new_id {
                track.track_id = new_id.increment();
            }
            track.score = new.score();
        });
    }

    pub(crate) fn update(&mut self, new: &STrack, frame_id: usize) {
        self.with_active_mut(|track| {
            kalman::update(
                &mut track.mean,
                &mut track.covariance,
                &new.to_tlwh().to_xyah(),
            );

            track.tracklet_len += 1;
            track.state = TrackState::Tracked;
            track.frame_id = frame_id;
            track.score = new.score();
        });
    }

    pub(crate) fn mark_lost(&mut self) {
        self.with_active_mut(|track| {
            track.state = TrackState::Lost;
        });
    }

    pub(crate) fn mark_removed(&mut self) {
        self.with_active_mut(|track| {
            track.state = TrackState::Removed;
        });
    }

    pub(crate) fn is_activated(&self) -> bool {
        matches!(self.inner, STrackInner::Active(_))
    }

    pub(crate) fn is_tracked(&self) -> bool {
        match self.inner {
            STrackInner::Active(ref active) => active.state == TrackState::Tracked,
            STrackInner::Inactive { .. } => false,
        }
    }

    pub(crate) fn is_lost(&self) -> bool {
        match self.inner {
            STrackInner::Active(ref active) => active.state == TrackState::Lost,
            STrackInner::Inactive { .. } => panic!("STrack has not been activated yet"),
        }
    }

    pub fn det_idx(&self) -> Option<usize> {
        match &self.inner {
            STrackInner::Active(active) => active.det_idx,
            STrackInner::Inactive { det_idx, .. } => *det_idx,
        }
    }

    pub(crate) fn set_det_idx(&mut self, idx: Option<usize>) {
        match self.inner {
            STrackInner::Active(ref mut active) => active.det_idx = idx,
            STrackInner::Inactive {
                ref mut det_idx, ..
            } => *det_idx = idx,
        }
    }

    pub fn track_id(&self) -> usize {
        match &self.inner {
            STrackInner::Active(active) => active.track_id,
            STrackInner::Inactive { .. } => panic!("STrack has not been activated yet"),
        }
    }

    pub fn frame_id(&self) -> usize {
        match &self.inner {
            STrackInner::Active(active) => active.frame_id,
            STrackInner::Inactive { .. } => panic!("STrack has not been activated yet"),
        }
    }

    pub fn start_frame(&self) -> usize {
        match &self.inner {
            STrackInner::Active(active) => active.start_frame,
            STrackInner::Inactive { .. } => panic!("STrack has not been activated yet"),
        }
    }

    pub fn score(&self) -> f32 {
        match &self.inner {
            STrackInner::Active(active) => active.score,
            STrackInner::Inactive { score, .. } => *score,
        }
    }

    pub fn to_tlwh(&self) -> Tlwh {
        match &self.inner {
            STrackInner::Active(active) => {
                let (cx, cy, a, h) = (
                    active.mean[0],
                    active.mean[1],
                    active.mean[2],
                    active.mean[3],
                );
                let w = a * h;
                Tlwh([cx - w / 2.0, cy - h / 2.0, w, h])
            }
            STrackInner::Inactive { bbox, .. } => bbox.clone(),
        }
    }

    pub fn to_tlbr(&self) -> Tlbr {
        self.to_tlwh().to_tlbr()
    }

    fn with_active_mut(&mut self, f: impl FnOnce(&mut ActiveSTrack)) {
        if let STrackInner::Active(active) = &mut self.inner {
            f(active);
        } else {
            panic!("STrack has not been activated yet");
        }
    }
}
