use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub bounds: Rect,
    pub rotation: f32,
}
impl Frame {
    pub fn center(self) -> Pos {
        self.bounds.center()
    }
    pub fn to_world(self, p: Pos) -> Pos {
        p.rotate(self.center(), self.rotation)
    }
    pub fn to_local(self, p: Pos) -> Pos {
        p.rotate(self.center(), -self.rotation)
    }
    pub fn corners(self) -> [Pos; 4] {
        self.bounds.corners().map(|p| self.to_world(p))
    }
    pub fn world_bounds(self) -> Rect {
        self.map_bounds(self.bounds)
    }
    pub fn map_bounds(self, bounds: Rect) -> Rect {
        Rect::enclosing(bounds.corners().map(|p| self.to_world(p)))
    }
    pub fn transform(self) -> Transform {
        let c = self.center();
        Transform::from_rotate_at(self.rotation.to_degrees(), c.x, c.y)
    }
}
impl Pos {
    pub fn rotate(self, center: Pos, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        let x = self.x - center.x;
        let y = self.y - center.y;
        Self::new(center.x + x * cos - y * sin, center.y + x * sin + y * cos)
    }
}
impl Rect {
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            w: (self.x + self.w).max(other.x + other.w) - x,
            h: (self.y + self.h).max(other.y + other.h) - y,
        }
    }
    pub fn intersects(self, other: Self) -> bool {
        self.x <= other.x + other.w
            && self.x + self.w >= other.x
            && self.y <= other.y + other.h
            && self.y + self.h >= other.y
    }
    pub fn center(self) -> Pos {
        Pos::new(self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn enclosing(points: [Pos; 4]) -> Self {
        let mut low = points[0];
        let mut high = low;
        for p in points {
            low.x = low.x.min(p.x);
            low.y = low.y.min(p.y);
            high.x = high.x.max(p.x);
            high.y = high.y.max(p.y);
        }
        Self::between(low, high)
    }
}

impl Mark {
    pub fn intersects(&self, rect: Rect) -> bool {
        if self.tool == Tool::Redact {
            return false;
        }
        let pad = if self.tool == Tool::Text {
            0.
        } else {
            self.style.width / 2.
        };
        let padded = Rect {
            x: rect.x - pad,
            y: rect.y - pad,
            w: rect.w + pad * 2.,
            h: rect.h + pad * 2.,
        };
        if !self.world_bounds().intersects(padded) {
            return false;
        }
        if (self.tool == Tool::Text || self.style.fill != Fill::None)
            && self.contains_body(rect.center())
        {
            return true;
        }
        let frame = self.frame();
        let edges: Vec<(Pos, Pos)> = if self.tool == Tool::Text {
            let corners = frame.corners();
            (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect()
        } else {
            self.path()
                .map(|p| {
                    shapes::edges(&p)
                        .into_iter()
                        .map(|(a, b)| (frame.to_world(a), frame.to_world(b)))
                        .collect()
                })
                .unwrap_or_default()
        };
        edges.into_iter().any(|(a, b)| {
            if padded.contains(a, 0.) || padded.contains(b, 0.) {
                return true;
            }
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            for x in [padded.x, padded.x + padded.w] {
                if dx.abs() > 0.0001 {
                    let t = (x - a.x) / dx;
                    let y = a.y + dy * t;
                    if (0. ..=1.).contains(&t) && y >= padded.y && y <= padded.y + padded.h {
                        return true;
                    }
                }
            }
            for y in [padded.y, padded.y + padded.h] {
                if dy.abs() > 0.0001 {
                    let t = (y - a.y) / dy;
                    let x = a.x + dx * t;
                    if (0. ..=1.).contains(&t) && x >= padded.x && x <= padded.x + padded.w {
                        return true;
                    }
                }
            }
            false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marquee_uses_rotated_geometry_and_transparent_interiors_pass_through() {
        let mut mark = Mark::new(
            Tool::Rectangle,
            Pos::new(100., 100.),
            Style {
                width: 8.,
                ..Default::default()
            },
        );
        mark.points[1] = Pos::new(200., 120.);
        mark.rotation = std::f32::consts::FRAC_PI_4;
        assert!(
            !mark.intersects(Rect {
                x: 108.,
                y: 140.,
                w: 8.,
                h: 8.
            }),
            "Do not select an empty corner of the rotated axis-aligned bounds"
        );
        assert!(mark.intersects(Rect {
            x: 130.,
            y: 90.,
            w: 10.,
            h: 10.
        }));
        mark.rotation = 0.;
        mark.points = vec![Pos::new(20., 20.), Pos::new(300., 240.)];
        let inside = Rect {
            x: 100.,
            y: 100.,
            w: 20.,
            h: 20.,
        };
        assert!(!mark.intersects(inside));
        mark.style.fill = Fill::Soft;
        assert!(mark.intersects(inside));
        assert!(mark.intersects(Rect {
            x: 0.,
            y: 0.,
            w: 350.,
            h: 280.
        }));
        mark.tool = Tool::Redact;
        assert!(!mark.intersects(inside));
    }
}
