use nalgebra::Vector2 as Vec2;
use rayon::prelude::*;

use super::fit_bez::fit_bezier_curve;
use crate::components::shapes::bezier::BezierCurve;

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