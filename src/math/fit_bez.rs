use nalgebra::{DMatrix, Matrix4, Vector2 as Vec2};

use crate::components::shapes::bezier::BezierCurve;

pub fn fit_bezier_curve(points: &[Vec2<f64>]) -> BezierCurve {
    return match best_fit(points) {
        [
            start, 
            control1, 
            control2, 
            end
        ] => BezierCurve { 
            start, 
            control1,
            control2, 
            end 
        }
    }
}

fn solve_for_b(a: &DMatrix<f64>) -> DMatrix<f64> {
    const EPSILON: f64 = 1e-10;
    let det = a.determinant();

    if det.abs() < EPSILON {
        // Matrix is nearly singular, use pseudo-inverse
        a.clone().pseudo_inverse(EPSILON).unwrap_or_else(|_| {
            eprintln!("Warning: Pseudo-inverse calculation failed. Returning original matrix.");
            a.clone()
        })
    } else {
        // Matrix is invertible, use standard inverse
        a.clone().try_inverse().unwrap_or_else(|| {
            eprintln!("Warning: Standard inverse calculation failed. Using pseudo-inverse.");
            a.clone().pseudo_inverse(EPSILON).unwrap_or_else(|_| {
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

#[inline]
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