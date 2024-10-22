use bevy_ecs::system::Query;

use crate::{Points, Spline};

use crate::components::shapes::spline::create_bezier_spline;


// any entities that have a Points attached are in the process of being drawn/edited
pub fn sys_update_spline(mut query: Query<(&mut Spline, &mut Points)>) {
    let (mut spline, points) = query.single_mut();
    spline.bez_spline = create_bezier_spline(&points.points, 100);
}

