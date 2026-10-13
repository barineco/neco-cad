use crate::{sqrt, CadElementIdentity, CpuPrimitive, CpuProjection, CpuProjectionFailure, Point2};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickKind {
    Face,
    Edge,
    Trim,
    Vertex,
    ControlPoint,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CpuPick {
    identity: CadElementIdentity,
    kind: PickKind,
    screen_position: Point2,
    depth: f64,
}
impl CpuPick {
    pub fn identity(&self) -> &CadElementIdentity {
        &self.identity
    }
    pub fn kind(&self) -> PickKind {
        self.kind
    }
    pub fn screen_position(&self) -> Point2 {
        self.screen_position
    }
    pub fn depth(&self) -> f64 {
        self.depth
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickTolerance {
    pixels: f64,
}
impl PickTolerance {
    pub fn new(pixels: f64) -> Result<Self, CpuProjectionFailure> {
        if pixels.is_finite() && pixels >= 0. {
            Ok(Self { pixels })
        } else {
            Err(CpuProjectionFailure::InvalidPickTolerance)
        }
    }
    pub fn pixels(self) -> f64 {
        self.pixels
    }
}
fn kind(id: &CadElementIdentity) -> PickKind {
    match id {
        CadElementIdentity::Face(_) => PickKind::Face,
        CadElementIdentity::Edge(_) => PickKind::Edge,
        CadElementIdentity::Trim(_) => PickKind::Trim,
        CadElementIdentity::Vertex(_) => PickKind::Vertex,
        CadElementIdentity::ControlPoint { .. } => PickKind::ControlPoint,
        _ => PickKind::Face,
    }
}
fn priority(k: PickKind) -> u8 {
    match k {
        PickKind::Vertex => 0,
        PickKind::ControlPoint => 1,
        PickKind::Trim => 2,
        PickKind::Edge => 3,
        PickKind::Face => 4,
    }
}
fn distance(p: Point2, q: Point2) -> f64 {
    let x = p.x() - q.x();
    let y = p.y() - q.y();
    sqrt(x * x + y * y)
}
fn cross(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x())
}
pub fn pick_cpu(
    projection: &CpuProjection,
    pixel: Point2,
    tolerance: PickTolerance,
) -> Result<Option<CpuPick>, CpuProjectionFailure> {
    let mut best: Option<(CpuPick, f64)> = None;
    for primitive in projection.primitives() {
        let (identity, screen_position, depth, distance_to_hit) = match primitive {
            CpuPrimitive::Point {
                identity,
                position,
                depth,
            } => {
                let d = distance(pixel, *position);
                if d > tolerance.pixels() {
                    continue;
                }
                (identity.clone(), *position, *depth, d)
            }
            CpuPrimitive::Segment {
                identity,
                start,
                end,
                depth,
            } => {
                let dx = end.x() - start.x();
                let dy = end.y() - start.y();
                let denom = dx * dx + dy * dy;
                let t = if denom == 0. {
                    0.
                } else {
                    (((pixel.x() - start.x()) * dx + (pixel.y() - start.y()) * dy) / denom)
                        .clamp(0., 1.)
                };
                let q = Point2::new(start.x() + t * dx, start.y() + t * dy)
                    .map_err(CpuProjectionFailure::CadView)?;
                let d = distance(pixel, q);
                if d > tolerance.pixels() {
                    continue;
                }
                (identity.clone(), q, (1. - t) * depth[0] + t * depth[1], d)
            }
            CpuPrimitive::Triangle {
                identity,
                vertices,
                depth,
            } => {
                let area = cross(vertices[0], vertices[1], vertices[2]);
                if area == 0. {
                    continue;
                }
                let w0 = cross(pixel, vertices[1], vertices[2]) / area;
                let w1 = cross(pixel, vertices[2], vertices[0]) / area;
                let w2 = cross(pixel, vertices[0], vertices[1]) / area;
                if !(w0 >= 0. && w1 >= 0. && w2 >= 0. || w0 <= 0. && w1 <= 0. && w2 <= 0.) {
                    continue;
                }
                (
                    identity.clone(),
                    pixel,
                    w0 * depth[0] + w1 * depth[1] + w2 * depth[2],
                    0.,
                )
            }
        };
        if depth <= 0. {
            continue;
        }
        let candidate = CpuPick {
            kind: kind(&identity),
            identity,
            screen_position,
            depth,
        };
        let better = match &best {
            None => true,
            Some((old, old_distance)) => {
                (
                    candidate.depth.total_cmp(&old.depth),
                    distance_to_hit.total_cmp(old_distance),
                    priority(candidate.kind).cmp(&priority(old.kind)),
                    candidate.identity.cmp(&old.identity),
                ) < (
                    core::cmp::Ordering::Equal,
                    core::cmp::Ordering::Equal,
                    core::cmp::Ordering::Equal,
                    core::cmp::Ordering::Equal,
                )
            }
        };
        if better {
            best = Some((candidate, distance_to_hit));
        }
    }
    Ok(best.map(|x| x.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::test_projection;
    use alloc::string::String;

    fn triangle(vertices: [Point2; 3]) -> CpuPrimitive {
        CpuPrimitive::Triangle {
            identity: CadElementIdentity::Face(String::from("face")),
            vertices,
            depth: [2.0, 2.0, 2.0],
        }
    }

    #[test]
    fn triangle_picking_is_invariant_under_winding() {
        let pixel = Point2::new(1.0, 1.0).unwrap();
        let ccw = triangle([
            Point2::new(0.0, 0.0).unwrap(),
            Point2::new(2.0, 0.0).unwrap(),
            Point2::new(0.0, 2.0).unwrap(),
        ]);
        let cw = triangle([
            Point2::new(0.0, 0.0).unwrap(),
            Point2::new(0.0, 2.0).unwrap(),
            Point2::new(2.0, 0.0).unwrap(),
        ]);
        let tolerance = PickTolerance::new(0.0).unwrap();
        let left = pick_cpu(&test_projection(ccw), pixel, tolerance)
            .unwrap()
            .unwrap();
        let right = pick_cpu(&test_projection(cw), pixel, tolerance)
            .unwrap()
            .unwrap();
        assert_eq!(left.identity(), right.identity());
        assert_eq!(left.depth(), right.depth());
    }
}
