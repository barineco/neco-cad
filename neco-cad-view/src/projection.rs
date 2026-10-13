use crate::{
    cross, dot, normalize, sub, CadElementIdentity, CadViewFailure, CadViewModel, Camera,
    CheckedRay3, Point2, Projection, Viewport, Visibility,
};
use alloc::string::String;
#[cfg(test)]
use alloc::vec;
use alloc::vec::Vec;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleRange {
    near: f64,
    far: f64,
}
impl VisibleRange {
    pub fn new(near: f64, far: f64) -> Result<Self, CpuProjectionFailure> {
        if near.is_finite() && far.is_finite() && near >= 0.0 && far > near {
            Ok(Self { near, far })
        } else {
            Err(CpuProjectionFailure::InvalidVisibleRange)
        }
    }
    pub fn near(self) -> f64 {
        self.near
    }
    pub fn far(self) -> f64 {
        self.far
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum CpuPrimitive {
    Triangle {
        identity: CadElementIdentity,
        vertices: [Point2; 3],
        depth: [f64; 3],
    },
    Segment {
        identity: CadElementIdentity,
        start: Point2,
        end: Point2,
        depth: [f64; 2],
    },
    Point {
        identity: CadElementIdentity,
        position: Point2,
        depth: f64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct CpuProjection {
    model_identity: crate::ViewModelIdentity,
    viewport: Viewport,
    visible_range: VisibleRange,
    primitives: Vec<CpuPrimitive>,
}
#[cfg(test)]
pub(crate) fn test_projection(primitive: CpuPrimitive) -> CpuProjection {
    CpuProjection {
        model_identity: crate::ViewModelIdentity::test(),
        viewport: Viewport::new(4, 4).unwrap(),
        visible_range: VisibleRange::new(0.0, 4.0).unwrap(),
        primitives: vec![primitive],
    }
}

#[cfg(feature = "test-fixtures")]
#[derive(Clone, Debug)]
pub struct CpuProjectionFixture {
    model_identity: crate::ViewModelIdentity,
    viewport: Viewport,
    visible_range: VisibleRange,
    primitives: Vec<CpuPrimitive>,
}

#[cfg(feature = "test-fixtures")]
impl CpuProjectionFixture {
    pub fn from_model(model: &CadViewModel, visible_range: VisibleRange) -> Self {
        Self {
            model_identity: model.identity().clone(),
            viewport: model.viewport(),
            visible_range,
            primitives: Vec::new(),
        }
    }

    pub fn new(
        model_identity: crate::ViewModelIdentity,
        viewport: Viewport,
        visible_range: VisibleRange,
    ) -> Self {
        Self {
            model_identity,
            viewport,
            visible_range,
            primitives: Vec::new(),
        }
    }

    pub fn with_primitive(mut self, primitive: CpuPrimitive) -> Self {
        self.primitives.push(primitive);
        self
    }

    pub fn into_projection(self) -> CpuProjection {
        CpuProjection {
            model_identity: self.model_identity,
            viewport: self.viewport,
            visible_range: self.visible_range,
            primitives: self.primitives,
        }
    }
}
impl CpuProjection {
    pub fn model_identity(&self) -> &crate::ViewModelIdentity {
        &self.model_identity
    }
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
    pub fn visible_range(&self) -> VisibleRange {
        self.visible_range
    }
    pub fn primitives(&self) -> &[CpuPrimitive] {
        &self.primitives
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum CpuProjectionFailure {
    CadView(CadViewFailure),
    PixelOutsideViewport,
    InvalidVisibleRange,
    InvalidPickTolerance,
    BehindCamera { identity: CadElementIdentity },
    NonProjectableTopology { identity: CadElementIdentity },
    MissingTriangleIdentity { triangle_index: usize },
}
fn frame(c: Camera) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let f = normalize(sub(c.target().array(), c.eye().array()));
    let r = normalize(cross(f, c.up().array()));
    (f, r, cross(r, f))
}
fn camera_point(model: &CadViewModel, p: [f64; 3]) -> [f64; 3] {
    let (f, r, q) = frame(model.camera());
    let d = sub(p, model.camera().eye().array());
    [dot(d, r), dot(d, q), dot(d, f)]
}
fn project_camera(
    model: &CadViewModel,
    p: [f64; 3],
) -> Result<(Point2, f64), CpuProjectionFailure> {
    let aspect = model.viewport().aspect_ratio();
    let (nx, ny) = match model.camera().projection() {
        Projection::Orthographic => {
            let h = model.camera().vertical_span().unwrap() / 2.0;
            (p[0] / h / aspect, p[1] / h)
        }
        Projection::Perspective { .. } => {
            if p[2] <= 0.0 {
                return Err(CpuProjectionFailure::BehindCamera {
                    identity: CadElementIdentity::Face(String::new()),
                });
            }
            let scale = model.camera().perspective_scale();
            (p[0] / p[2] / scale / aspect, p[1] / p[2] / scale)
        }
    };
    let w = model.viewport().width_pixels() as f64;
    let h = model.viewport().height_pixels() as f64;
    Ok((
        Point2::new(w * (nx + 1.0) / 2.0 - 0.5, h * (1.0 - ny) / 2.0 - 0.5)
            .map_err(CpuProjectionFailure::CadView)?,
        p[2],
    ))
}
fn clip_polygon_at(points: &[[f64; 3]], plane: [f64; 4]) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    if points.is_empty() {
        return out;
    }
    let distance = |p: [f64; 3]| plane[0] * p[0] + plane[1] * p[1] + plane[2] * p[2] + plane[3];
    let mut previous = points[points.len() - 1];
    let mut previous_distance = distance(previous);
    for &current in points {
        let current_distance = distance(current);
        if (previous_distance >= 0.0) != (current_distance >= 0.0) {
            let t = previous_distance / (previous_distance - current_distance);
            out.push([
                previous[0] + t * (current[0] - previous[0]),
                previous[1] + t * (current[1] - previous[1]),
                previous[2] + t * (current[2] - previous[2]),
            ]);
        }
        if current_distance >= 0.0 {
            out.push(current);
        }
        previous = current;
        previous_distance = current_distance;
    }
    out
}
fn clip_planes(model: &CadViewModel, range: VisibleRange, extent: f64) -> Vec<[f64; 4]> {
    let mut planes = Vec::new();
    let minimum = match model.camera().projection() {
        Projection::Perspective { .. } if range.near == 0.0 => {
            let scale = model.camera().perspective_scale();
            let unit = f64::EPSILON;
            if scale > 1.0 {
                unit
            } else {
                unit / scale
            }
        }
        _ => range.near,
    };
    planes.push([0.0, 0.0, 1.0, -minimum]);
    planes.push([0.0, 0.0, -1.0, range.far]);
    if let Projection::Perspective { .. } = model.camera().projection() {
        let vertical = model.camera().perspective_scale();
        let horizontal = vertical * model.viewport().aspect_ratio();
        planes.extend([
            [1.0, 0.0, horizontal * extent, 0.0],
            [-1.0, 0.0, horizontal * extent, 0.0],
            [0.0, 1.0, vertical * extent, 0.0],
            [0.0, -1.0, vertical * extent, 0.0],
        ]);
    }
    planes
}
fn clipped_polygon(
    model: &CadViewModel,
    points: [[f64; 3]; 3],
    range: VisibleRange,
) -> Vec<[f64; 3]> {
    let mut polygon = points.to_vec();
    for plane in clip_planes(model, range, 1.0) {
        polygon = clip_polygon_at(&polygon, plane);
    }
    polygon
}
fn clipped_segment(
    model: &CadViewModel,
    a: [f64; 3],
    b: [f64; 3],
    range: VisibleRange,
) -> Option<([f64; 3], [f64; 3])> {
    let planes = clip_planes(model, range, 1.0);
    clip_segment_at_planes(a, b, &planes)
}

fn clip_segment_at_planes(
    mut a: [f64; 3],
    mut b: [f64; 3],
    planes: &[[f64; 4]],
) -> Option<([f64; 3], [f64; 3])> {
    for plane in planes {
        let distance = |point: [f64; 3]| {
            plane[0] * point[0] + plane[1] * point[1] + plane[2] * point[2] + plane[3]
        };
        let da = distance(a);
        let db = distance(b);
        if da < 0.0 && db < 0.0 {
            return None;
        }
        if (da < 0.0) != (db < 0.0) {
            let t = da / (da - db);
            let intersection = [
                a[0] + t * (b[0] - a[0]),
                a[1] + t * (b[1] - a[1]),
                a[2] + t * (b[2] - a[2]),
            ];
            if da < 0.0 {
                a = intersection;
            } else {
                b = intersection;
            }
        }
    }
    Some((a, b))
}
fn is_backface(model: &CadViewModel, values: [[f64; 3]; 3]) -> bool {
    view_space_backface(model.camera().projection(), values)
}

fn view_space_backface(projection: Projection, values: [[f64; 3]; 3]) -> bool {
    let normal = cross(sub(values[1], values[0]), sub(values[2], values[0]));
    let orientation = match projection {
        Projection::Orthographic => normal[2],
        Projection::Perspective { .. } => dot(normal, values[0]),
    };
    orientation >= 0.0
}
pub fn project_cpu_reference(
    model: &CadViewModel,
    range: VisibleRange,
) -> Result<CpuProjection, CpuProjectionFailure> {
    if matches!(model.camera().projection(), Projection::Perspective { .. })
        && model.camera().perspective_scale() == 0.0
    {
        return Err(CpuProjectionFailure::CadView(
            CadViewFailure::NonFiniteCoordinate,
        ));
    }
    let mut primitives = Vec::new();
    let verts = model.mesh().mesh().vertices();
    if model.display().faces() == Visibility::Visible {
        for (identity, triangle) in &model.geometry().faces {
            let polygon = clipped_polygon(
                model,
                [
                    camera_point(model, verts[triangle[0]]),
                    camera_point(model, verts[triangle[1]]),
                    camera_point(model, verts[triangle[2]]),
                ],
                range,
            );
            for index in 1..polygon.len().saturating_sub(1) {
                let values = [polygon[0], polygon[index], polygon[index + 1]];
                let mut points = [Point2::new(0.0, 0.0).unwrap(); 3];
                let mut depth = [0.0; 3];
                for i in 0..3 {
                    (points[i], depth[i]) =
                        project_camera(model, values[i]).map_err(|error| match error {
                            CpuProjectionFailure::BehindCamera { .. } => {
                                CpuProjectionFailure::BehindCamera {
                                    identity: identity.clone(),
                                }
                            }
                            error => error,
                        })?;
                }
                if model.display().backface_culling() && is_backface(model, values) {
                    continue;
                }
                primitives.push(CpuPrimitive::Triangle {
                    identity: identity.clone(),
                    vertices: points,
                    depth,
                });
            }
        }
    }
    for (identity, points) in &model.geometry().curves {
        let visible = match identity {
            CadElementIdentity::Edge(_) => model.display().edges(),
            CadElementIdentity::Trim(_) => model.display().trims(),
            _ => Visibility::Hidden,
        };
        if visible == Visibility::Visible {
            for pair in points.windows(2) {
                if let Some((a, b)) = clipped_segment(
                    model,
                    camera_point(model, pair[0]),
                    camera_point(model, pair[1]),
                    range,
                ) {
                    let (start, da) = project_camera(model, a)?;
                    let (end, db) = project_camera(model, b)?;
                    primitives.push(CpuPrimitive::Segment {
                        identity: identity.clone(),
                        start,
                        end,
                        depth: [da, db],
                    });
                }
            }
        }
    }
    for (identity, point) in &model.geometry().points {
        let visible = match identity {
            CadElementIdentity::Vertex(_) => model.display().vertices(),
            CadElementIdentity::ControlPoint { .. } => model.display().control_points(),
            _ => Visibility::Hidden,
        };
        let value = camera_point(model, *point);
        let inside = clip_planes(model, range, 1.0).iter().all(|plane| {
            plane[0] * value[0] + plane[1] * value[1] + plane[2] * value[2] + plane[3] >= 0.0
        });
        if visible == Visibility::Visible && inside {
            let (position, depth) = project_camera(model, value)?;
            primitives.push(CpuPrimitive::Point {
                identity: identity.clone(),
                position,
                depth,
            });
        }
    }
    Ok(CpuProjection {
        model_identity: model.identity().clone(),
        viewport: model.viewport(),
        visible_range: range,
        primitives,
    })
}
pub fn ray_from_viewport_point(
    model: &CadViewModel,
    pixel: Point2,
) -> Result<CheckedRay3, CpuProjectionFailure> {
    let w = model.viewport().width_pixels() as f64;
    let h = model.viewport().height_pixels() as f64;
    if pixel.x() < 0.0 || pixel.y() < 0.0 || pixel.x() >= w || pixel.y() >= h {
        return Err(CpuProjectionFailure::PixelOutsideViewport);
    }
    let nx = 2.0 * ((pixel.x() + 0.5) / w) - 1.0;
    let ny = 1.0 - 2.0 * ((pixel.y() + 0.5) / h);
    let (f, r, q) = frame(model.camera());
    let (origin, direction) = match model.camera().projection() {
        Projection::Orthographic => {
            let hh = model.camera().vertical_span().unwrap() / 2.0;
            let hw = hh * model.viewport().aspect_ratio();
            let e = model.camera().eye().array();
            (
                [
                    e[0] + nx * hw * r[0] + ny * hh * q[0],
                    e[1] + nx * hw * r[1] + ny * hh * q[1],
                    e[2] + nx * hw * r[2] + ny * hh * q[2],
                ],
                f,
            )
        }
        Projection::Perspective { .. } => {
            let hh = model.camera().perspective_scale();
            let hw = hh * model.viewport().aspect_ratio();
            (
                model.camera().eye().array(),
                normalize([
                    f[0] + nx * hw * r[0] + ny * hh * q[0],
                    f[1] + nx * hw * r[1] + ny * hh * q[1],
                    f[2] + nx * hw * r[2] + ny * hh * q[2],
                ]),
            )
        }
    };
    CheckedRay3::try_new(origin, direction, 0.0).map_err(|error| {
        CpuProjectionFailure::CadView(CadViewFailure::GeometryFailure {
            error: error.into(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_near_zero_segment_remains_finite_after_clipping() {
        let planes = [[0.0, 0.0, 1.0, -f64::EPSILON], [0.0, 0.0, -1.0, 4.0]];
        let (a, b) = clip_segment_at_planes([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], &planes).unwrap();
        assert!(a.iter().all(|value| value.is_finite()));
        assert!(b.iter().all(|value| value.is_finite()));
        assert!(a[2] > 0.0);
    }

    #[test]
    fn fixture_segment_clips_against_side_plane_without_polygon_closure() {
        let planes = [[1.0, 0.0, 1.0, 0.0], [-1.0, 0.0, 1.0, 0.0]];
        let (a, b) = clip_segment_at_planes([-2.0, 0.0, 1.0], [0.0, 0.0, 1.0], &planes).unwrap();
        assert!(a[0] < 0.0);
        assert!(a[0] > -2.0);
        assert_eq!(a[1], 0.0);
        assert_eq!(a[2], 1.0);
        assert_eq!(b, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn fixture_culling_uses_view_space_normal_orientation() {
        let front = [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]];
        let back = [front[0], front[2], front[1]];
        assert!(view_space_backface(Projection::Orthographic, front));
        assert!(!view_space_backface(Projection::Orthographic, back));
        assert!(view_space_backface(
            Projection::Perspective {
                vertical_field_of_view_radians: core::f64::consts::FRAC_PI_2,
            },
            front
        ));
        assert!(!view_space_backface(
            Projection::Perspective {
                vertical_field_of_view_radians: core::f64::consts::FRAC_PI_2,
            },
            back
        ));
    }
}
