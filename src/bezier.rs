use nalgebra::{DMatrix, Matrix4, Vector2 as Vec2};
use rayon::prelude::*;

pub struct BezierCurve {
    pub start: Vec2<f64>,
    pub control1: Vec2<f64>,
    pub control2: Vec2<f64>,
    pub end: Vec2<f64>,
}

pub fn create_bezier_spline(points: &[Vec2<f64>], size: usize) -> Vec<BezierCurve> {
    let mut spline_parts: Vec<_> = Vec::new();
    let mut last_vec_i = 0;
    let mut last_vec2 = points[0];

    for i in 1..points.len() {
        let vec2 = points[i];
        if last_vec2.metric_distance(&vec2) > size as f64 {
            let sub_points = points[last_vec_i..(i+1)].to_vec();
            spline_parts.push(sub_points);
            last_vec2 = vec2;
            last_vec_i = i;
        }
    }

    if spline_parts.is_empty() {
        spline_parts.push(points.to_vec());
    } else {
        let sub_points = points[last_vec_i..].to_vec();
        spline_parts.push(sub_points);
    }

    let mut spline: Vec<BezierCurve> = spline_parts.into_par_iter()
        .map(|sub_points| fit_bezier_curve(&sub_points))
        .collect();

    if spline.len() > 1 {
        smooth_spline(&mut spline);
    }

    spline
}

fn fit_bezier_curve(points: &[Vec2<f64>]) -> BezierCurve {
    // This function will contain the least squares fitting logic from your original bezier.rs
    // We'll modify it slightly to return a BezierCurve struct
    let control_points = vec_to_bezier_control_points(points);
    BezierCurve {
        start: control_points[0],
        control1: control_points[1],
        control2: control_points[2],
        end: control_points[3],
    }
}

fn smooth_spline(spline: &mut Vec<BezierCurve>) {
    if spline.len() <= 1 {
        return;
    }

    // First pass: compute adjustments
    let adjustments: Vec<_> = spline.windows(2)
        .map(|window| {
            let prev = &window[0];
            let curr = &window[1];
            let prev_tangent = prev.end - prev.control2;
            (curr.start - prev.end, curr.start + prev_tangent - curr.control1)
        })
        .collect();

    // Second pass: apply adjustments in parallel
    spline.par_iter_mut().skip(1).zip(adjustments.par_iter())
        .for_each(|(curve, &(start_adj, control1_adj))| {
            curve.start += start_adj;
            curve.control1 += control1_adj;
        });
}

pub fn vec_to_bezier_control_points(points: &[Vec2<f64>]) -> [Vec2<f64>; 4] {
     return best_fit(points);
}

fn solve_for_b(a: &DMatrix<f64>) -> DMatrix<f64> {
    let epsilon = f64::from(1e-10);
    let det = a.determinant();

    if det.abs() < epsilon {
        // Matrix is nearly singular, use pseudo-inverse
        a.clone().pseudo_inverse(epsilon).unwrap_or_else(|_| {
            eprintln!("Warning: Pseudo-inverse calculation failed. Returning original matrix.");
            a.clone()
        })
    } else {
        // Matrix is invertible, use standard inverse
        a.clone().try_inverse().unwrap_or_else(|| {
            eprintln!("Warning: Standard inverse calculation failed. Using pseudo-inverse.");
            a.clone().pseudo_inverse(epsilon).unwrap_or_else(|_| {
                eprintln!("Warning: Pseudo-inverse calculation also failed. Returning original matrix.");
                a.clone()
            })
        })
    }
}

fn best_fit(points: &[Vec2<f64>]) -> [Vec2<f64>; 4] {
    let m = m();
    let min_v = m.try_inverse().expect("min_V error");

    let u = u(points);
    let ut = u.transpose();

    let x = x(points);
    let y = y(points);

    let a = ut.clone() * u;
    let b = solve_for_b(&a.clone());

    let c = min_v * b;
    let d = c * ut.clone();
    let e = d.clone() * x;
    let f = d.clone() * y;

    let mut p_arr = [Vec2::new(0.0,0.0); 4];

    for i in 0..4 {
        let x = e[(i, 0)];
        let y = f[(i,0)];

        let p = Vec2::new(x, y);
        p_arr[i] = p;
    }

    return p_arr;
}

fn m() -> Matrix4<f64> {
    Matrix4::new(
        -1.0, 3.0, -3.0, 1.0,
        3.0, -6.0, 3.0, 0.0,
        -3.0, 3.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 0.0,
    )
}

fn u(points: &[Vec2<f64>]) -> DMatrix<f64> {
    let npls = normalized_path_lengths(points);

    let mut u = DMatrix::from_element(points.len(), 4, 0.0);

    
    for i in 0..npls.len() {
        u[(i,0)] = npls[i].powi(3);
        u[(i,1)] = npls[i].powi(2);
        u[(i,2)] = npls[i].powi(1);
        u[(i,3)] = npls[i].powi(0);
    }
    return u;
}

fn x(points: &[Vec2<f64>]) -> DMatrix<f64> {
    let mut x = DMatrix::from_element(points.len(), 1, 0.0);

    for i in 0..points.len() {
        x[(i, 0)] = points[i].x;
    }

    return x;
}

fn y(points: &[Vec2<f64>]) -> DMatrix<f64> {
    let mut y = DMatrix::from_element(points.len(), 1, 0.0);

    for i in 0..points.len() {
        y[(i, 0)] = points[i].y;
    }
    return y;
}

fn normalized_path_lengths(points: &[Vec2<f64>]) -> Vec<f64> {
    let mut path_length = vec![0.0; points.len()];

    path_length[0] = 0.0;

    for i in 1..points.len() {
        let p1 = points[i];
        let p2 = points[i - 1];
        let distance = ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt();
        path_length[i] = path_length[i - 1] + distance;
    }

    // TODO: make this a little rustier and less Javaish
    let mut zpl = vec![0.0; path_length.len()];
    for i in 0..path_length.len() {
        zpl[i] = path_length[i] / path_length[path_length.len() - 1];
    }

    return zpl;

}