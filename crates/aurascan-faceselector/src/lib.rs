use std::cmp::Ordering;

#[derive(Debug)]
pub struct FaceSelector {
    score_margin: f32,
    missing_threshold: usize,

    current_id: Option<u64>,
    missing: usize,
}

impl FaceSelector {
    /// Create a new [`FaceSelector`].
    ///
    /// - `score_margin` is the added margin to the calculated score to
    /// switch to a new face, if the tracked face is present in the
    /// detections but a face with a higher score is present.
    ///
    /// - `missing_threshold` is the threshold of how many frames (calls to
    /// [`FaceSelector::select`]) the face must be missing to drop the
    /// selection and choose a new best one.
    pub fn new(score_margin: f32, missing_threshold: usize) -> Self {
        Self {
            score_margin,
            missing_threshold,

            current_id: None,
            missing: 0,
        }
    }

    /// Select a face from the detections and return the ID.
    ///
    /// Returns `None` if `detections` is empty or if the current selected
    /// face is not present in the detections and it hasn't been dropped
    /// yet.
    pub fn select<D: FaceDetection>(
        &mut self,
        mut detections: impl Iterator<Item = D> + Clone,
        frame_width: f32,
        frame_height: f32,
    ) -> Option<u64> {
        debug_assert!(frame_width > 0.0 && frame_height > 0.0);
        debug_assert_detections(detections.clone(), frame_width, frame_height);

        let best = detections.clone().max_by(|a, b| {
            let a_score = face_score(a, frame_width, frame_height);
            let b_score = face_score(b, frame_width, frame_height);
            a_score.partial_cmp(&b_score).unwrap_or(Ordering::Equal)
        });

        // No face selected, use best if available.
        let Some(current_id) = self.current_id else {
            self.current_id = best.map(|detection| detection.id());
            return self.current_id;
        };

        // Search for a detection with the current ID selected.
        let maybe_detection = detections.find(|detection| detection.id() == current_id);
        let Some(matching_detection) = maybe_detection else {
            self.missing += 1;

            // If missing for too long, select the best face as the new one being tracked.
            if self.missing > self.missing_threshold {
                self.missing = 0;
                self.current_id = best.map(|detection| detection.id());
                return self.current_id;
            }

            return None;
        };

        // Not missing.
        self.missing = 0;

        // If there is a face with a score above the margin to switch, use
        // this one.
        if let Some(best) = best {
            let matching_score = face_score(&matching_detection, frame_width, frame_height);
            let best_score = face_score(&best, frame_width, frame_height);
            let is_above_margin = best_score > matching_score + self.score_margin;
            if best.id() != matching_detection.id() && is_above_margin {
                self.current_id = Some(best.id());
            }
        }

        self.current_id
    }
}

/// The bounding box and score of a face detection.
pub trait FaceDetection {
    /// X coordinate.
    fn x(&self) -> f32;

    /// Y coordinate.
    fn y(&self) -> f32;

    /// Width.
    fn w(&self) -> f32;

    /// Height.
    fn h(&self) -> f32;

    /// The face detection score.
    fn score(&self) -> f32;

    /// The ID of the face detection.
    fn id(&self) -> u64;
}

/// Compute the face score.
///
/// Takes into account by weighting the area of the face, how close is it to
/// the center and the detection score.
///
/// The return value is in the range `[0.0, 1.0]`, the higher the value the
/// better the score.
///
/// The area of the face has an importance of 60% of the score, the centrality a 30%
/// and the detection score 10%.
fn face_score<D: FaceDetection>(detection: &D, frame_width: f32, frame_height: f32) -> f32 {
    const AREA_WEIGHT: f32 = 0.6;
    const CENTRALITY_WEIGHT: f32 = 0.3;
    const SCORE_WEIGHT: f32 = 0.1;

    debug_assert!(
        (AREA_WEIGHT + CENTRALITY_WEIGHT + SCORE_WEIGHT - 1.0).abs() < f32::EPSILON,
        "face score weights must sum up to 1.0"
    );

    let area = area_ratio(detection, frame_width, frame_height);
    let centrality = centrality(detection, frame_width, frame_height);

    (AREA_WEIGHT * area)
        + (CENTRALITY_WEIGHT * centrality)
        + (SCORE_WEIGHT * detection.score().clamp(0.0, 1.0))
}

/// Computes how close the detection is to the center of the frame.
///
/// The return value is in the range `[0.0, 1.0]`, where `1.0` means the box
/// is at the center of the frame.
fn centrality<D: FaceDetection>(detection: &D, frame_width: f32, frame_height: f32) -> f32 {
    let frame_center_x = frame_width / 2.0;
    let frame_center_y = frame_height / 2.0;

    let center_x = detection.x() + (detection.w() / 2.0);
    let center_y = detection.y() + (detection.h() / 2.0);

    // Offset from the frame center, normalized to [-1.0, 1.0].
    let offset_x = (center_x - frame_center_x) / frame_center_x;
    let offset_y = (center_y - frame_center_y) / frame_center_y;

    // Distance from the face's center to the frame center.
    let distance = (offset_x * offset_x + offset_y * offset_y).sqrt();

    // Clamp or the result can be negative if the face is close to a corner.
    1.0 - distance.min(1.0)
}

/// The detection area in relation to the frame size.
///
/// The return value is in the range `[0.0, 1.0]`, where `1.0` means the box
/// fills the entire frame.
fn area_ratio<D: FaceDetection>(detection: &D, frame_width: f32, frame_height: f32) -> f32 {
    (detection.w() * detection.h()) / (frame_width * frame_height)
}

/// Asserts that [`FaceDetection`]s satisfy the invariants assumed by this
/// crate.
fn debug_assert_detections<D: FaceDetection>(
    detections: impl Iterator<Item = D>,
    frame_width: f32,
    frame_height: f32,
) {
    detections.for_each(|detection| {
        debug_assert_detection(&detection, frame_width, frame_height);
    });
}

/// Asserts that a [`FaceDetection`] satisfies the invariants assumed by this
/// crate.
///
/// This is a no-op in release builds.
fn debug_assert_detection<D: FaceDetection>(detection: &D, frame_width: f32, frame_height: f32) {
    debug_assert!(
        detection.x().is_finite()
            && detection.y().is_finite()
            && detection.w().is_finite()
            && detection.h().is_finite(),
        "detection geometry must be finite, got x={}, y={}, w={}, h={}",
        detection.x(),
        detection.y(),
        detection.w(),
        detection.h(),
    );

    debug_assert!(
        detection.w() >= 0.0 && detection.h() >= 0.0,
        "detection width and height must be non-negative, got w={}, h={}",
        detection.w(),
        detection.h(),
    );

    debug_assert!(
        detection.score().is_finite() && (0.0..=1.0).contains(&detection.score()),
        "detection score must be in [0.0, 1.0], got {}",
        detection.score(),
    );

    debug_assert!(
        detection.x() >= 0.0
            && detection.y() >= 0.0
            && detection.x() + detection.w() <= frame_width
            && detection.y() + detection.h() <= frame_height,
        "detection bounding box must lie within the frame ({}x{}), \
         got x={}, y={}, w={}, h={}",
        frame_width,
        frame_height,
        detection.x(),
        detection.y(),
        detection.w(),
        detection.h(),
    );
}
