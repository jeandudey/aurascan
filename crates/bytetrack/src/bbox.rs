use crate::Measurement;
use nalgebra::SVector;

#[derive(Debug, Clone)]
pub struct Tlwh(pub [f32; 4]);

impl Tlwh {
    pub fn to_xyah(&self) -> Measurement {
        let [x, y, w, h] = self.0;
        let a = w / h;
        Measurement::new(x + w / 2.0, y + h / 2.0, a, h)
    }

    pub fn to_tlbr(&self) -> Tlbr {
        let [x, y, w, h] = self.0;
        Tlbr([x, y, x + w, y + h])
    }
}

#[derive(Debug, Clone)]
pub struct Tlbr(pub [f32; 4]);

impl Tlbr {
    pub fn width(&self) -> f32 {
        let [x1, _, x2, _] = self.0;
        x2 - x1
    }

    pub fn height(&self) -> f32 {
        let [_, y1, _, y2] = self.0;
        y2 - y1
    }

    pub fn iou(&self, other: &Tlbr) -> f32 {
        let [self_x1, self_y1, self_x2, self_y2] = self.0;
        let [other_x1, other_y1, other_x2, other_y2] = other.0;
        let xx1 = self_x1.max(other_x1);
        let yy1 = self_y1.max(other_y1);
        let xx2 = self_x2.min(other_x2);
        let yy2 = self_y2.min(other_y2);

        let w = (xx2 - xx1).max(0.0);
        let h = (yy2 - yy1).max(0.0);
        let inter = w * h;

        let area_self = self.width() * self.height();
        let area_other = other.width() * other.height();
        let area_union = area_self + area_other - inter;
        if area_union <= 0.0 {
            0.0
        } else {
            inter / area_union
        }
    }
}

#[derive(Debug, Clone)]
pub enum BoundingBox {
    /// Top-left bottom-right format: `[x1, y1, x2, y2]`.
    Tlbr(SVector<f32, 4>),
    /// Top-left width-height format: `[x, y, w, h]`.
    Tlwh(SVector<f32, 4>),
}

impl BoundingBox {
    pub fn from_tlbr(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::Tlbr(SVector::<f32, 4>::new(x1, y1, x2, y2))
    }

    pub fn from_tlwh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::Tlwh(SVector::<f32, 4>::new(x, y, w, h))
    }

    pub fn x1(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(v) => v[0],
            BoundingBox::Tlwh(v) => v[0],
        }
    }

    pub fn y1(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(v) => v[1],
            BoundingBox::Tlwh(v) => v[1],
        }
    }

    pub fn x2(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(v) => v[2],
            BoundingBox::Tlwh(v) => v[0] + v[2],
        }
    }

    pub fn y2(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(v) => v[3],
            BoundingBox::Tlwh(v) => v[1] + v[3],
        }
    }

    pub fn width(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(tlbr) => tlbr[2] - tlbr[0],
            BoundingBox::Tlwh(tlwh) => tlwh[2],
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            BoundingBox::Tlbr(tlbr) => tlbr[3] - tlbr[1],
            BoundingBox::Tlwh(tlwh) => tlwh[3],
        }
    }

    pub fn iou(&self, other: &Self) -> f32 {
        let xx1 = self.x1().max(other.x1());
        let yy1 = self.y1().max(other.y1());
        let xx2 = self.x2().min(other.x2());
        let yy2 = self.y2().min(other.y2());

        let w = (xx2 - xx1).max(0.0);
        let h = (yy2 - yy1).max(0.0);
        let inter = w * h;

        let area_self = self.width() * self.height();
        let area_other = other.width() * other.height();
        let area_union = area_self + area_other - inter;
        if area_union <= 0.0 {
            0.0
        } else {
            inter / area_union
        }
    }
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub score: f32,
    pub bbox: BoundingBox,
}

impl Detection {
    pub fn to_tlwh(&self) -> Tlwh {
        match &self.bbox {
            BoundingBox::Tlbr(tlbr) => {
                Tlwh([tlbr[0], tlbr[1], tlbr[2] - tlbr[0], tlbr[3] - tlbr[1]])
            }
            BoundingBox::Tlwh(tlwh) => Tlwh([tlwh[0], tlwh[1], tlwh[2], tlwh[3]]),
        }
    }

    pub fn to_tlbr(&self) -> Tlbr {
        match &self.bbox {
            BoundingBox::Tlbr(tlbr) => Tlbr([tlbr[0], tlbr[1], tlbr[2], tlbr[3]]),
            BoundingBox::Tlwh(tlwh) => {
                Tlbr([tlwh[0], tlwh[1], tlwh[0] + tlwh[2], tlwh[1] + tlwh[3]])
            }
        }
    }

    //pub fn to_tlwh(&self) -> Self {
    //    Self {
    //        score: self.score,
    //        bbox: self.bbox.to_tlwh(),
    //    }
    //}
}
